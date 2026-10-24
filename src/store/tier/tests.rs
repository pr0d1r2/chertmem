use std::error::Error as StdError;

use crate::store::tests::{Scratch, Test, WEEK, chunk, event, stored};
use crate::store::{Error, Filter, Lookup, Mode, Ref, State, Stats, Store};

/// The tier the catalog gives `path`, `None` when it lists no such row.
fn state(store: &Store, path: &str) -> Result<Option<State>, Error> {
    let all = store.partitions(&Filter::default())?;
    Ok(all.into_iter().find(|p| p.path == path).map(|p| p.state))
}

/// The bodies `refs` resolve to; `None` for a dropped one.
fn bodies(
    store: &mut Store,
    refs: &[Ref],
) -> Result<Vec<Option<String>>, Box<dyn StdError>> {
    let found = store.get(refs)?.into_iter().map(|l| match l {
        Lookup::Found(o) => Some(o.body),
        Lookup::Dropped(_) => None,
    });
    Ok(found.collect())
}

/// 3 rows in the first week, 1 in the next; the refs and week 0's path.
fn fill(store: &mut Store) -> Result<(Vec<Ref>, String), Box<dyn StdError>> {
    let mut refs = Vec::new();
    for (ts, body) in [(5, "a"), (6, "b"), (WEEK + 1, "c"), (7, "d")] {
        refs.push(stored(store.append(&event(ts, None, body))?)?);
    }
    Ok((refs, chunk(store, "19700101-7d")))
}

fn some(names: &[&str]) -> Vec<Option<String>> {
    names.iter().map(|n| Some((*n).to_owned())).collect()
}

#[test]
fn seal_makes_a_partition_warm_and_read_only() -> Test {
    let s = Scratch::new("tier-seal")?;
    let mut store = s.open(Mode::Read)?;
    let (refs, path) = fill(&mut store)?;
    store.seal(&path)?;
    store.seal(&path)?;
    assert_eq!(state(&store, &path)?, Some(State::Warm));
    let late = store.append(&event(8, None, "too late"));
    assert_eq!(late, Err(Error::ReadOnly { path: path.clone() }));
    let mut other = s.open(Mode::Read)?;
    let refused = other.append(&event(9, None, "elsewhere"));
    assert_eq!(refused, Err(Error::ReadOnly { path }));
    assert_eq!(bodies(&mut store, &refs)?, some(&["a", "b", "c", "d"]));
    assert_eq!(store.generation()?, 0);
    Ok(())
}

/// The stats of every partition the catalog lists.
fn stats(store: &Store) -> Result<Vec<Stats>, Error> {
    let all = store.partitions(&Filter::default())?;
    Ok(all.into_iter().map(|p| p.stats).collect())
}

#[test]
fn compact_makes_a_partition_cold_and_keeps_its_stats() -> Test {
    let s = Scratch::new("tier-compact")?;
    let mut store = s.open(Mode::Read)?;
    let (_, path) = fill(&mut store)?;
    let sums = stats(&store)?;
    store.compact(&path)?;
    assert_eq!(state(&store, &path)?, Some(State::Cold));
    assert!(!s.data().join(&path).exists());
    assert!(s.data().join(format!("{path}.zst")).is_file());
    assert_eq!(stats(&store)?, sums);
    Ok(())
}

#[test]
fn a_cold_partition_reads_through_its_thawed_copy() -> Test {
    let s = Scratch::new("tier-thaw")?;
    let mut store = s.open(Mode::Read)?;
    let (refs, path) = fill(&mut store)?;
    let before = store.observations(&Filter::default())?;
    store.compact(&path)?;
    let mut fresh = s.open(Mode::Read)?;
    assert_eq!(fresh.observations(&Filter::default())?, before);
    assert!(s.data().join("thawed").join(&path).is_file());
    assert_eq!(bodies(&mut fresh, &refs)?, some(&["a", "b", "c", "d"]));
    let late = fresh.append(&event(8, None, "too late"));
    assert_eq!(late, Err(Error::ReadOnly { path }));
    Ok(())
}

#[test]
fn compact_without_dictionary_writes_none() -> Test {
    let s = Scratch::new("tier-nodict")?;
    let mut cfg = s.config()?;
    cfg.compression.zstd.dictionary = false;
    let mut store = Store::open(&cfg, &s.project(), Mode::Read)?;
    let (refs, path) = fill(&mut store)?;
    store.compact(&path)?;
    assert!(!s.data().join(format!("{path}.dict")).exists());
    let mut fresh = Store::open(&cfg, &s.project(), Mode::Read)?;
    assert_eq!(bodies(&mut fresh, &refs)?, some(&["a", "b", "c", "d"]));
    Ok(())
}

#[test]
fn drop_answers_dropped_never_another_row() -> Test {
    let s = Scratch::new("tier-drop")?;
    let mut store = s.open(Mode::Read)?;
    let (refs, path) = fill(&mut store)?;
    store.drop(&path)?;
    assert!(!s.data().join(&path).exists());
    assert_eq!(state(&store, &path)?, None);
    let c = Some("c".to_owned());
    assert_eq!(bodies(&mut store, &refs)?, [None, None, c, None]);
    assert_eq!(store.generation()?, 1);
    let again = store.append(&event(5, None, "same ms as a"));
    assert_eq!(again, Err(Error::ReadOnly { path: path.clone() }));
    store.drop(&path)?;
    assert_eq!(store.generation()?, 1);
    Ok(())
}

#[test]
fn drop_removes_every_file_of_a_cold_partition() -> Test {
    let s = Scratch::new("tier-drop-cold")?;
    let mut store = s.open(Mode::Read)?;
    let (refs, path) = fill(&mut store)?;
    store.compact(&path)?;
    assert_eq!(bodies(&mut store, &refs)?, some(&["a", "b", "c", "d"]));
    store.drop(&path)?;
    for end in [".zst", ".dict", ""] {
        assert!(!s.data().join(format!("{path}{end}")).exists(), "{end}");
    }
    assert!(!s.data().join("thawed").join(&path).exists());
    assert_eq!(bodies(&mut store, &refs)?.iter().flatten().count(), 1);
    Ok(())
}

#[test]
fn every_rewrite_bumps_generation_and_a_seal_does_not() -> Test {
    let s = Scratch::new("tier-generation")?;
    let mut store = s.open(Mode::Read)?;
    let (_, path) = fill(&mut store)?;
    store.seal(&path)?;
    assert_eq!(store.generation()?, 0);
    store.compact(&path)?;
    assert_eq!(store.generation()?, 1);
    store.compact(&path)?;
    assert_eq!(store.generation()?, 1);
    Ok(())
}

#[test]
fn compacting_a_partition_whose_file_is_gone_is_an_io_error() -> Test {
    let s = Scratch::new("tier-compact-gone")?;
    let mut store = s.open(Mode::Read)?;
    let (_, path) = fill(&mut store)?;
    store.seal(&path)?;
    drop(store);
    std::fs::remove_file(s.data().join(&path))?;
    let mut store = s.open(Mode::Read)?;
    let e = store.compact(&path);
    assert!(matches!(e, Err(Error::Io { .. })), "{e:?}");
    assert_eq!(state(&store, &path)?, Some(State::Warm));
    Ok(())
}

#[test]
fn an_unknown_partition_is_an_error() -> Test {
    let s = Scratch::new("tier-unknown")?;
    let mut store = s.open(Mode::Read)?;
    let path = chunk(&store, "nowhere");
    assert!(matches!(store.seal(&path), Err(Error::Sqlite { .. })));
    assert!(matches!(store.compact(&path), Err(Error::Sqlite { .. })));
    assert!(matches!(store.drop(&path), Err(Error::Sqlite { .. })));
    Ok(())
}

#[test]
fn a_write_only_store_moves_no_tier() -> Test {
    let s = Scratch::new("tier-write-only")?;
    let mut hook = s.open(Mode::Write)?;
    let path = chunk(&hook, "19700101-7d");
    assert_eq!(hook.seal(&path), Err(Error::WriteOnly));
    assert_eq!(hook.compact(&path), Err(Error::WriteOnly));
    assert_eq!(hook.drop(&path), Err(Error::WriteOnly));
    assert_eq!(hook.repartition(), Err(Error::WriteOnly));
    Ok(())
}
