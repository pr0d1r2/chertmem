use std::error::Error as StdError;
use std::fs;

use rusqlite::types::Value;

use crate::store::tests::{Scratch, Test, WEEK, chunk, event, stored};
use crate::store::{Appended, Error, Lookup, Mode, Ref, Store};

type Dump = (Vec<String>, Vec<String>);

const PARTITIONS: &str = "SELECT path, dims, project, ide, t0, t1, shard,
  rows, bytes, state, checksum, spec, dropped FROM partitions ORDER BY path";
const REFS: &str = "SELECT r.project, r.ref, r.id, r.identity, p.path
  FROM refs r JOIN partitions p ON p.id = r.partition ORDER BY r.id";

/// Every row `sql` selects, its columns joined.
fn rows(s: &Scratch, sql: &str) -> Result<Vec<String>, Box<dyn StdError>> {
    let conn = s.sql("catalog.db")?;
    let mut stmt = conn.prepare(sql)?;
    let n = stmt.column_count();
    let found = stmt.query_map([], |r| {
        let cols =
            (0..n).map(|i| r.get::<_, Value>(i).map(|v| format!("{v:?}")));
        Ok(cols.collect::<Result<Vec<_>, _>>()?.join("|"))
    })?;
    Ok(found.collect::<Result<_, _>>()?)
}

/// The catalog by content: partitions and refs, keyed by path, not id.
fn dump(s: &Scratch) -> Result<Dump, Box<dyn StdError>> {
    Ok((rows(s, PARTITIONS)?, rows(s, REFS)?))
}

/// Weeks 0 (hot), 1 (warm) and 2 (cold); the refs.
fn tiers(store: &mut Store) -> Result<Vec<Ref>, Box<dyn StdError>> {
    let mut refs = Vec::new();
    for (ts, id) in [(5, "a"), (WEEK + 1, "b"), (2 * WEEK + 1, "c"), (6, "d")] {
        refs.push(stored(store.append(&event(ts, Some(id), id))?)?);
    }
    store.seal(&chunk(store, "19700108-7d"))?;
    store.compact(&chunk(store, "19700115-7d"))?;
    Ok(refs)
}

fn remove_catalog(s: &Scratch) {
    for end in ["", "-wal", "-shm"] {
        let _ = fs::remove_file(s.data().join(format!("catalog.db{end}")));
    }
}

#[test]
fn a_deleted_catalog_rebuilds_equal_to_the_one_before() -> Test {
    let s = Scratch::new("doctor-deleted")?;
    let mut store = s.open(Mode::Read)?;
    let refs = tiers(&mut store)?;
    drop(store);
    let before = dump(&s)?;
    remove_catalog(&s);
    let mut store = s.open(Mode::Read)?;
    store.rebuild()?;
    assert_eq!(dump(&s)?, before);
    let found = store.get(&refs)?;
    assert!(found.iter().all(|l| matches!(l, Lookup::Found(_))));
    let again = store.append(&event(7, Some("a"), "again"))?;
    assert_eq!(Some(again), refs.first().cloned().map(Appended::Stored));
    assert!(store.generation()? > 0);
    Ok(())
}

/// A row in week 3, the catalog copied aside, then 3 tiers and week 3
/// dropped; the dropped ref and the catalog before the crash.
fn stale(s: &Scratch) -> Result<(Ref, Dump), Box<dyn StdError>> {
    let mut store = s.open(Mode::Read)?;
    let gone = stored(store.append(&event(3 * WEEK + 1, None, "gone"))?)?;
    drop(store);
    fs::copy(s.data().join("catalog.db"), s.data().join("stale.db"))?;
    let mut store = s.open(Mode::Read)?;
    tiers(&mut store)?;
    store.drop(&chunk(&store, "19700122-7d"))?;
    drop(store);
    Ok((gone, dump(s)?))
}

#[test]
fn a_stale_catalog_rebuilds_equal_to_the_one_before() -> Test {
    let s = Scratch::new("doctor-stale")?;
    let (gone, before) = stale(&s)?;
    remove_catalog(&s);
    fs::rename(s.data().join("stale.db"), s.data().join("catalog.db"))?;
    let mut store = s.open(Mode::Read)?;
    store.rebuild()?;
    assert_eq!(dump(&s)?, before);
    let found = store.get(std::slice::from_ref(&gone))?;
    assert_eq!(found, [Lookup::Dropped(gone)]);
    Ok(())
}

#[test]
fn a_write_only_store_rebuilds_nothing() -> Test {
    let s = Scratch::new("doctor-write-only")?;
    let mut hook = s.open(Mode::Write)?;
    assert_eq!(hook.rebuild(), Err(Error::WriteOnly));
    Ok(())
}
