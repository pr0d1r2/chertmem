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

/// `path` with `~<n>` before `.db`: the `n`-th fresh partition of its
/// bucket (src/store V13).
fn fresh(path: &str, n: u32) -> String {
    let stem = path.strip_suffix(".db").unwrap_or(path);
    format!("{stem}~{n}.db")
}

/// The rows in the partition file at `path`.
fn count(s: &Scratch, path: &str) -> Result<u64, Box<dyn StdError>> {
    let sql = "SELECT count(*) FROM observations";
    Ok(s.sql(path)?.query_row(sql, [], |r| r.get(0))?)
}

/// A move of the partition at a path down the tiers.
type Retire = fn(&mut Store, &str) -> Result<(), Error>;

/// Fill, `retire` week 0, then append 1 more event in week 0 from a
/// reader and 1 from a hook: both are stored in its fresh partition,
/// hot, and every live ref still resolves (src/store V13, B3).
fn lands_fresh(name: &str, retire: Retire, live: &[&str]) -> Test {
    let s = Scratch::new(name)?;
    let mut store = s.open(Mode::Read)?;
    let (mut refs, path) = fill(&mut store)?;
    retire(&mut store, &path)?;
    refs.push(stored(store.append(&event(8, None, "late"))?)?);
    let hook = s.open(Mode::Write)?.append(&event(9, None, "hook"));
    refs.push(stored(hook?)?);
    assert_eq!(state(&store, &fresh(&path, 1))?, Some(State::Hot));
    assert_eq!(count(&s, &fresh(&path, 1))?, 2);
    let mut reader = s.open(Mode::Read)?;
    let found = bodies(&mut reader, &refs)?;
    assert_eq!(found.iter().flatten().count(), live.len());
    assert_eq!(texts(&mut reader, &Filter::default())?, live);
    Ok(())
}

/// The body of every observation `filter` wants, in order.
fn texts(
    store: &mut Store,
    filter: &Filter,
) -> Result<Vec<String>, Box<dyn StdError>> {
    let rows = store.observations(filter)?;
    Ok(rows.into_iter().map(|o| o.body).collect())
}

#[test]
fn an_append_past_a_sealed_bucket_lands_in_a_fresh_partition() -> Test {
    let all = ["a", "b", "d", "late", "hook", "c"];
    lands_fresh("tier-fresh-seal", Store::seal, &all)
}

#[test]
fn an_append_past_a_compacted_bucket_lands_in_a_fresh_partition() -> Test {
    let all = ["a", "b", "d", "late", "hook", "c"];
    lands_fresh("tier-fresh-compact", Store::compact, &all)
}

#[test]
fn an_append_past_a_dropped_bucket_lands_in_a_fresh_partition() -> Test {
    let live = ["late", "hook", "c"];
    lands_fresh("tier-fresh-drop", Store::drop, &live)
}

#[test]
fn each_retired_fresh_partition_gives_way_to_the_next() -> Test {
    let s = Scratch::new("tier-fresh-next")?;
    let mut store = s.open(Mode::Read)?;
    let (_, path) = fill(&mut store)?;
    store.compact(&path)?;
    store.append(&event(8, None, "first fresh"))?;
    store.drop(&fresh(&path, 1))?;
    store.append(&event(9, None, "second fresh"))?;
    assert_eq!(count(&s, &fresh(&path, 2))?, 1);
    store.rebuild()?;
    let week = Filter {
        until: Some(WEEK),
        ..Filter::default()
    };
    assert_eq!(texts(&mut store, &week)?, ["a", "b", "d", "second fresh"]);
    Ok(())
}

/// The refusal of a seal or compaction of `path` before `end`.
fn open_until(path: &str, end: u64) -> Result<(), Error> {
    let path = path.to_owned();
    Err(Error::Open { path, end })
}

#[test]
fn seal_and_compact_refuse_a_bucket_still_open() -> Test {
    let s = Scratch::new("tier-open")?;
    let mut store = s.open(Mode::Read)?;
    let (_, path) = fill(&mut store)?;
    let (open, end) = (open_until(&path, WEEK), WEEK);
    assert_eq!(store.seal_at(&path, end - 1), open);
    assert_eq!(store.compact_at(&path, 5), open);
    assert_eq!(state(&store, &path)?, Some(State::Hot));
    store.seal_at(&path, end)?;
    assert_eq!(store.compact_at(&path, end - 1), open);
    assert_eq!(state(&store, &path)?, Some(State::Warm));
    store.compact_at(&path, end)?;
    assert_eq!(state(&store, &path)?, Some(State::Cold));
    Ok(())
}

#[test]
fn seal_and_compact_without_a_clock_take_any_bucket() -> Test {
    let s = Scratch::new("tier-no-clock")?;
    let mut store = s.open(Mode::Read)?;
    store.append(&event(1 << 47, None, "far ahead"))?;
    let all = store.partitions(&Filter::default())?;
    let path = all.first().ok_or("no partition")?.path.clone();
    store.compact(&path)?;
    assert_eq!(state(&store, &path)?, Some(State::Cold));
    Ok(())
}

#[test]
fn seal_makes_a_partition_warm_and_read_only() -> Test {
    let s = Scratch::new("tier-seal")?;
    let mut store = s.open(Mode::Read)?;
    let (refs, path) = fill(&mut store)?;
    let was = store.generation()?;
    store.seal(&path)?;
    store.seal(&path)?;
    assert_eq!(state(&store, &path)?, Some(State::Warm));
    store.append(&event(8, None, "too late"))?;
    s.open(Mode::Read)?.append(&event(9, None, "elsewhere"))?;
    assert_eq!(count(&s, &path)?, 3);
    assert_eq!(count(&s, &fresh(&path, 1))?, 2);
    assert_eq!(bodies(&mut store, &refs)?, some(&["a", "b", "c", "d"]));
    assert_eq!(store.generation()?, was);
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
    let was = store.generation()?;
    store.drop(&path)?;
    assert!(!s.data().join(&path).exists());
    assert_eq!(state(&store, &path)?, None);
    let c = Some("c".to_owned());
    assert_eq!(bodies(&mut store, &refs)?, [None, None, c, None]);
    let once = store.generation()?;
    store.drop(&path)?;
    assert!(once > was && store.generation()? == once);
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
    let was = store.generation()?;
    store.seal(&path)?;
    assert_eq!(store.generation()?, was);
    store.compact(&path)?;
    let once = store.generation()?;
    store.compact(&path)?;
    assert!(once > was && store.generation()? == once);
    Ok(())
}

#[test]
fn compacting_a_partition_whose_file_is_gone_is_an_io_error() -> Test {
    let s = Scratch::new("tier-compact-gone")?;
    let mut store = s.open(Mode::Read)?;
    let (_, path) = fill(&mut store)?;
    drop(store);
    std::fs::remove_file(s.data().join(&path))?;
    let mut store = s.open(Mode::Read)?;
    store.seal(&path)?;
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
