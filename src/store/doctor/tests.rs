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

/// Rows in weeks 3 and 4, the catalog copied aside, then 3 tiers and
/// week 3 dropped; the dropped ref and the catalog before the crash.
fn stale(s: &Scratch) -> Result<(Ref, Dump), Box<dyn StdError>> {
    let mut store = s.open(Mode::Read)?;
    let gone = stored(store.append(&event(3 * WEEK + 1, None, "gone"))?)?;
    store.append(&event(4 * WEEK + 1, None, "kept"))?;
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

/// Append into week 1 (warm) and week 2 (cold) under a catalog that
/// was deleted and so calls them nothing, then hot.
fn appends_after_a_lost_catalog(s: &Scratch) -> Test {
    let mut store = s.open(Mode::Read)?;
    tiers(&mut store)?;
    drop(store);
    remove_catalog(s);
    let mut store = s.open(Mode::Read)?;
    for (ts, week) in [(WEEK + 2, "19700108-7d"), (2 * WEEK + 2, "19700115-7d")]
    {
        let path = chunk(&store, week);
        let refused = store.append(&event(ts, None, "late"));
        assert_eq!(refused, Err(Error::ReadOnly { path }));
    }
    Ok(())
}

#[test]
fn a_sealed_file_refuses_writes_even_when_the_catalog_forgot() -> Test {
    let s = Scratch::new("doctor-forgot")?;
    appends_after_a_lost_catalog(&s)
}

#[test]
fn a_dropped_row_wins_over_a_stray_file_at_its_path() -> Test {
    let s = Scratch::new("doctor-stray")?;
    let mut store = s.open(Mode::Read)?;
    let a = stored(store.append(&event(5, None, "a"))?)?;
    store.append(&event(WEEK + 1, None, "b"))?;
    let week0 = chunk(&store, "19700101-7d");
    let week1 = chunk(&store, "19700108-7d");
    store.drop(&week0)?;
    drop(store);
    fs::copy(s.data().join(&week1), s.data().join(&week0))?;
    let mut store = s.open(Mode::Read)?;
    store.rebuild()?;
    let found = store.get(std::slice::from_ref(&a))?;
    assert_eq!(found, [Lookup::Dropped(a)]);
    Ok(())
}

#[test]
fn a_write_only_store_rebuilds_nothing() -> Test {
    let s = Scratch::new("doctor-write-only")?;
    let mut hook = s.open(Mode::Write)?;
    assert_eq!(hook.rebuild(), Err(Error::WriteOnly));
    Ok(())
}

/// Far past the clock, as a burst of bumps in 1 ms may leave it.
const AHEAD: &str = "UPDATE meta SET value = 9000000000000
  WHERE key = 'generation'";

/// Every `generation` a cache-like reader saw while weeks 0 and 1 were
/// dropped, the counter first pushed past the clock.
fn seen(s: &Scratch) -> Result<Vec<u64>, Box<dyn StdError>> {
    let mut store = s.open(Mode::Read)?;
    store.append(&event(5, None, "a"))?;
    store.append(&event(WEEK + 1, None, "b"))?;
    let mut seen = vec![store.generation()?];
    s.sql("catalog.db")?.execute(AHEAD, [])?;
    for week in ["19700101-7d", "19700108-7d"] {
        store.drop(&chunk(&store, week))?;
        seen.push(store.generation()?);
    }
    Ok(seen)
}

#[test]
fn a_generation_seen_before_a_lost_catalog_never_comes_back() -> Test {
    let s = Scratch::new("doctor-generation")?;
    let seen = seen(&s)?;
    remove_catalog(&s);
    let mut store = s.open(Mode::Read)?;
    let fresh = store.generation()?;
    store.rebuild()?;
    let after = [fresh, store.generation()?];
    assert!(
        after.iter().all(|g| !seen.contains(g)),
        "{seen:?} {after:?}"
    );
    assert!(fresh < store.generation()?);
    Ok(())
}

/// A store in 2 shards: rows in 1 ms split across 2 files.
fn sharded(s: &Scratch) -> Result<Store, Box<dyn StdError>> {
    let mut cfg = s.config()?;
    cfg.partition.shards = 2;
    Ok(Store::open(&cfg, &s.project(), Mode::Read)?)
}

const HOLDER: &str = "SELECT p.path FROM refs r
  JOIN partitions p ON p.id = r.partition WHERE r.ref = ?1";

/// 6 rows in 1 ms, then the shard holding the first, shortest ref
/// dropped; every ref it held.
fn drop_first(s: &Scratch) -> Result<Vec<Ref>, Box<dyn StdError>> {
    let mut store = sharded(s)?;
    let mut refs = Vec::new();
    for body in ["a", "b", "c", "d", "e", "f"] {
        refs.push(stored(store.append(&event(5, None, body))?)?);
    }
    let first = refs.first().ok_or("no ref")?.as_str().to_owned();
    let catalog = s.sql("catalog.db")?;
    let path: String = catalog.query_row(HOLDER, [first], |r| r.get(0))?;
    store.drop(&path)?;
    let found = store.get(&refs)?;
    let gone = refs.into_iter().zip(found);
    let gone = gone.filter(|(_, l)| matches!(l, Lookup::Dropped(_)));
    Ok(gone.map(|(r, _)| r).collect())
}

/// 12 appends in the dropped rows' ms: the live shard takes them, the
/// dropped one refuses; none is issued a ref in `gone`.
fn burst(store: &mut Store, gone: &[Ref]) -> Test {
    for n in 0..12 {
        match store.append(&event(5, None, &format!("burst {n}"))) {
            Ok(a) => assert!(!gone.contains(&stored(a)?)),
            Err(e) => assert!(matches!(e, Error::ReadOnly { .. }), "{e}"),
        }
    }
    Ok(())
}

#[test]
fn a_dropped_ref_stays_dropped_across_a_lost_catalog() -> Test {
    let s = Scratch::new("doctor-tombstone")?;
    let gone = drop_first(&s)?;
    remove_catalog(&s);
    let mut store = sharded(&s)?;
    store.rebuild()?;
    let dropped: Vec<_> = gone.iter().cloned().map(Lookup::Dropped).collect();
    assert_eq!(store.get(&gone)?, dropped);
    burst(&mut store, &gone)?;
    assert_eq!(store.get(&gone)?, dropped);
    Ok(())
}
