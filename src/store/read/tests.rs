use std::error::Error as StdError;
use std::fs;

use crate::config::{Config, Dimension, Span, Unit};
use crate::store::tests::{Scratch, Test, WEEK, event, stored};
use crate::store::{Error, Filter, Lookup, Mode, Observation, Ref, Store};

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

/// The body 1 ref resolves to, or why not.
fn body(store: &mut Store, r: &Ref) -> Result<String, Box<dyn StdError>> {
    match store.get(std::slice::from_ref(r))?.pop() {
        Some(Lookup::Found(o)) if &o.reference == r => Ok(o.body),
        other => Err(format!("{r}: {other:?}").into()),
    }
}

#[test]
fn another_projects_ref_resolves_when_this_one_lacks_it() -> Test {
    let s = Scratch::new("ref-cross")?;
    let mut a = s.open_other("alpha")?;
    let r = stored(a.append(&event(7, None, "from alpha"))?)?;
    let mut b = s.open_other("beta")?;
    assert_eq!(body(&mut b, &r)?, "from alpha");
    Ok(())
}

/// Projects alpha and beta, each with 1 row under the same ref: the
/// first row of a millisecond gets the same 8 chars anywhere.
fn twins(s: &Scratch) -> Result<(Store, Store, Ref), Box<dyn StdError>> {
    let (mut a, mut b) = (s.open_other("alpha")?, s.open_other("beta")?);
    let ra = stored(a.append(&event(7, None, "alpha's"))?)?;
    let rb = stored(b.append(&event(7, None, "beta's"))?)?;
    if ra != rb {
        return Err(format!("{ra} != {rb}").into());
    }
    Ok((a, b, ra))
}

#[test]
fn the_current_project_wins() -> Test {
    let s = Scratch::new("ref-current")?;
    let (mut a, mut b, r) = twins(&s)?;
    assert_eq!(body(&mut a, &r)?, "alpha's");
    assert_eq!(body(&mut b, &r)?, "beta's");
    Ok(())
}

#[test]
fn a_ref_in_two_other_projects_names_both() -> Test {
    let s = Scratch::new("ref-ambiguous")?;
    let (a, b, r) = twins(&s)?;
    let mut projects = vec![a.project().to_owned(), b.project().to_owned()];
    projects.sort();
    let reference = r.to_string();
    let want = Error::Ambiguous {
        reference,
        projects,
    };
    assert_eq!(s.open_other("gamma")?.get(&[r]).err(), Some(want));
    Ok(())
}

/// Rows in 1 second (src/store V8).
const BURST: u64 = 100_000;

#[test]
fn every_ref_of_a_burst_resolves_to_its_own_row() -> Test {
    let s = Scratch::new("ref-burst")?;
    let mut store = s.open(Mode::Read)?;
    let start = 1_709_211_600_000;
    let mut issued = Vec::new();
    for i in 0..BURST {
        let e = event(start + i % 1000, None, &i.to_string());
        issued.push(stored(store.append(&e)?)?);
    }
    let got = store.get(&issued)?;
    for (i, found) in got.iter().enumerate() {
        let want = i.to_string();
        let ok = matches!(found, Lookup::Found(o) if o.body == want);
        assert!(ok, "{i}: {found:?}");
    }
    Ok(())
}

#[test]
fn generation_reads_the_catalog_counter() -> Test {
    let s = Scratch::new("generation")?;
    let mut store = s.open(Mode::Read)?;
    let was = store.generation()?;
    store.append(&event(5, None, "appends leave it"))?;
    assert_eq!(store.generation()?, was);
    let sql = "UPDATE meta SET value = 3 WHERE key = 'generation'";
    s.sql("catalog.db")?.execute(sql, [])?;
    assert_eq!(store.generation()?, 3);
    let hook = s.open(Mode::Write)?;
    assert_eq!(hook.generation(), Err(Error::WriteOnly));
    Ok(())
}
