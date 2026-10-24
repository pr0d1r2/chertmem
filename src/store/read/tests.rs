use std::error::Error as StdError;
use std::fs;

use crate::config::{Config, Dimension, Span, Unit};
use crate::store::tests::{Scratch, Test, WEEK, event, stored};
use crate::store::{Error, Filter, Mode, Observation, Ref, Store};

/// A store under `cfg` for the scratch project.
fn open(
    s: &Scratch,
    cfg: &Config,
    mode: Mode,
) -> Result<Store, Box<dyn StdError>> {
    Ok(Store::open(cfg, &s.project(), mode)?)
}

/// Daily buckets, 4 shards, time only: nothing like the defaults.
fn reshaped(s: &Scratch) -> Result<Config, Box<dyn StdError>> {
    let mut cfg = s.config()?;
    cfg.partition.by = vec![Dimension::Time];
    cfg.partition.time_bucket = Span {
        amount: 1,
        unit: Unit::Days,
    };
    cfg.partition.shards = 4;
    Ok(cfg)
}

fn of(project: &str) -> Filter {
    Filter {
        project: Some(project.to_owned()),
        ..Filter::default()
    }
}

fn refs(found: &[Observation]) -> Vec<Ref> {
    found.iter().map(|o| o.reference.clone()).collect()
}

#[test]
fn a_config_change_still_finds_old_rows() -> Test {
    let s = Scratch::new("read-reconfig")?;
    let mut old = s.open(Mode::Write)?;
    let a = stored(old.append(&event(5, None, "old week"))?)?;
    let mut new = open(&s, &reshaped(&s)?, Mode::Read)?;
    let b = stored(new.append(&event(6, None, "new day"))?)?;
    assert_eq!(refs(&new.observations(&Filter::default())?), [a, b]);
    let all = new.partitions(&Filter::default())?;
    assert_eq!(all.len(), 2);
    assert!(all.iter().any(|p| p.spec.contains("time_bucket=7d")));
    Ok(())
}

#[test]
fn bounds_prune_partitions_without_opening_them() -> Test {
    let s = Scratch::new("read-prune")?;
    let mut store = s.open(Mode::Read)?;
    store.append(&event(5, None, "week 0"))?;
    let late = stored(store.append(&event(3 * WEEK, None, "week 3"))?)?;
    let early = format!("chunks/{}/19700101-7d.db", store.project());
    drop(store);
    // A pruned file is never opened: garbage there cannot fail the read.
    fs::write(s.data().join(&early), "not a database")?;
    let mut store = s.open(Mode::Read)?;
    let since = Filter {
        since: Some(2 * WEEK),
        ..Filter::default()
    };
    assert_eq!(refs(&store.observations(&since)?), [late]);
    assert!(store.observations(&Filter::default()).is_err());
    Ok(())
}

#[test]
fn rows_outside_the_window_or_project_stay_out() -> Test {
    let s = Scratch::new("read-window")?;
    let mut store = s.open(Mode::Read)?;
    for ts in [5, 6, 7] {
        store.append(&event(ts, None, "x"))?;
    }
    let window = Filter {
        since: Some(6),
        until: Some(7),
        ..of(store.project())
    };
    let found = store.observations(&window)?;
    assert_eq!(found.iter().map(|o| o.ts).collect::<Vec<_>>(), [6]);
    assert!(store.observations(&of("someone-else"))?.is_empty());
    assert!(store.partitions(&of("someone-else"))?.is_empty());
    Ok(())
}

#[test]
fn a_partition_without_project_dim_serves_every_project() -> Test {
    let s = Scratch::new("read-no-project-dim")?;
    let mut store = open(&s, &reshaped(&s)?, Mode::Read)?;
    store.append(&event(5, None, "x"))?;
    let mine = of(store.project());
    assert_eq!(store.partitions(&mine)?.len(), 1);
    assert_eq!(store.observations(&mine)?.len(), 1);
    assert_eq!(store.partitions(&of("someone-else"))?.len(), 1);
    assert!(store.observations(&of("someone-else"))?.is_empty());
    Ok(())
}

#[test]
fn a_missing_partition_file_reads_as_empty() -> Test {
    let s = Scratch::new("read-missing")?;
    let mut store = s.open(Mode::Read)?;
    store.append(&event(5, None, "x"))?;
    let path = format!("chunks/{}/19700101-7d.db", store.project());
    drop(store);
    fs::remove_file(s.data().join(&path))?;
    let mut store = s.open(Mode::Read)?;
    assert!(store.observations(&Filter::default())?.is_empty());
    assert!(!s.data().join(&path).exists());
    Ok(())
}

#[test]
fn a_write_only_store_reads_nothing() -> Test {
    let s = Scratch::new("read-write-only")?;
    let mut store = s.open(Mode::Write)?;
    let all = Filter::default();
    assert_eq!(store.partitions(&all).err(), Some(Error::WriteOnly));
    assert_eq!(store.observations(&all).err(), Some(Error::WriteOnly));
    Ok(())
}
