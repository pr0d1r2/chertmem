//! The window alone, and V1 & V2 through a handle with a real second
//! process writing.

use std::error::Error as StdError;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use super::{Fresh, Window};
use crate::cache::index::{Index, Rowless};
use crate::cache::tests::{
    DAY, HOUR, NOW, Recorder, START, Scratch, Test, config_at, event, from,
    lend, recorded, rewrite_elsewhere, same,
};
use crate::cache::{Handle, Source};
use crate::store::{Filter, Mode, Observation, Store};

/// A window from [`START`] with every partition loaded into it and
/// `index`.
fn fed(
    store: &mut Store,
    index: &mut dyn Index,
) -> Result<Window, Box<dyn StdError>> {
    let mut window = Window::begin(store, START)?;
    for path in window.paths() {
        window.sync(store, index, &path)?;
    }
    Ok(window)
}

/// [`fed`] with no index.
fn loaded(store: &mut Store) -> Result<Window, Box<dyn StdError>> {
    fed(store, &mut Rowless)
}

/// The rows the window lends `q`, cloned; `None`: declined.
fn answer(window: &Window, q: &Filter) -> Option<Vec<Observation>> {
    window.view(q).map(|rows| rows.to_vec())
}

#[test]
fn begin_lists_the_partitions_of_the_window_only() -> Test {
    let s = Scratch::new("window-list")?;
    let mut store = s.open(Mode::Read)?;
    store.append(&event(START - 30 * DAY, "far before"))?;
    store.append(&event(NOW - HOUR, "inside"))?;
    let window = Window::begin(&store, START)?;
    let inside = store.partitions(&from(&store, START))?;
    let paths: Vec<String> = inside.into_iter().map(|p| p.path).collect();
    assert_eq!(window.paths(), paths);
    assert_eq!(paths.len(), 1);
    Ok(())
}

#[test]
fn sync_loads_a_partition_and_answer_keeps_ts_id_order() -> Test {
    let s = Scratch::new("window-sync")?;
    let mut store = s.open(Mode::Read)?;
    store.append(&event(NOW - HOUR, "b"))?;
    store.append(&event(NOW - 3 * HOUR, "a"))?;
    store.append(&event(START - DAY, "before"))?;
    let window = loaded(&mut store)?;
    let q = from(&store, START);
    let rows = answer(&window, &q).ok_or("declined")?;
    assert_eq!(rows, store.observations(&q)?);
    let bodies: Vec<&str> = rows.iter().map(|o| o.body.as_str()).collect();
    assert_eq!(bodies, ["a", "b"]);
    Ok(())
}

#[test]
fn answer_declines_what_the_window_may_not_hold() -> Test {
    let s = Scratch::new("window-decline")?;
    let store = s.open(Mode::Read)?;
    let window = Window::begin(&store, START)?;
    let mut other = from(&store, START);
    other.project = None;
    for q in [Filter::default(), from(&store, START - 1), other] {
        assert_eq!(answer(&window, &q), None, "{q:?}");
    }
    assert_eq!(answer(&window, &from(&store, START)), Some(Vec::new()));
    Ok(())
}

#[test]
fn refresh_tails_another_connections_rows_by_seq_not_ulid() -> Test {
    let s = Scratch::new("window-refresh")?;
    let (mut store, mut hook) = (s.open(Mode::Read)?, s.open(Mode::Write)?);
    hook.append(&event(NOW - HOUR, "first"))?;
    let mut window = loaded(&mut store)?;
    // An earlier ts: a lower ULID than the row kept, a higher seq.
    hook.append(&event(NOW - 2 * HOUR, "earlier ts, appended later"))?;
    // A bucket the window did not list yet.
    hook.append(&event(START + HOUR, "new bucket"))?;
    assert_eq!(window.refresh(&mut store, &mut Rowless)?, Fresh::Current);
    let q = from(&store, START);
    let rows = answer(&window, &q).ok_or("declined")?;
    assert_eq!((rows.len(), window.paths().len()), (3, 2));
    assert_eq!(rows, store.observations(&q)?);
    Ok(())
}

#[test]
fn a_new_generation_makes_the_window_stale() -> Test {
    let s = Scratch::new("window-stale")?;
    let mut store = s.open(Mode::Read)?;
    store.append(&event(NOW - HOUR, "late"))?;
    let mut window = loaded(&mut store)?;
    rewrite_elsewhere(&s, &store)?;
    assert_eq!(window.refresh(&mut store, &mut Rowless)?, Fresh::Stale);
    Ok(())
}

/// What a store never does but a tail may meet: the row at a kept
/// `(ts, id)` read again at a higher seq, its body changed.
const MOVE: &str = "UPDATE observations SET seq = seq + 1000, body = 'moved'
  WHERE body = 'kept'";

#[test]
fn a_row_replaced_at_its_key_reaches_the_index_with_the_old_one() -> Test {
    let s = Scratch::new("window-replace")?;
    let mut store = s.open(Mode::Read)?;
    store.append(&event(NOW - HOUR, "kept"))?;
    let mut index = Recorder::new(1);
    let mut window = fed(&mut store, &mut index)?;
    for p in store.partitions(&from(&store, START))? {
        s.sql(&p.path)?.execute(MOVE, [])?;
    }
    assert_eq!(window.refresh(&mut store, &mut index)?, Fresh::Current);
    let bodies: Vec<&String> = index.rows.values().collect();
    assert_eq!(bodies, ["moved"]);
    assert_eq!(index.replaced, ["kept"]);
    let rows = answer(&window, &from(&store, START)).ok_or("declined")?;
    assert_eq!(rows, store.observations(&from(&store, START))?);
    Ok(())
}

const WRITER: &str = "CHERT_CACHE_WRITER";
const WRITES: &str = "CHERT_CACHE_WRITES";

/// Not a check of its own: the second process of
/// `cached_equals_uncached_with_a_writer_process`, which re-invokes this
/// test binary to run it. It appends 1 event per ts in `WRITES` through
/// its own store; without `WRITER` it does nothing.
#[test]
fn writer_process() -> Test {
    let Some(base) = std::env::var_os(WRITER).map(PathBuf::from) else {
        return Ok(());
    };
    let root = base.join("project");
    let mut store = Store::open(&config_at(&base)?, &root, Mode::Write)?;
    for ts in std::env::var(WRITES)?.split(',') {
        store.append(&event(ts.parse()?, "from the writer process"))?;
    }
    Ok(())
}

/// [`writer_process`] in a second process appending at `ts`.
fn writer(s: &Scratch, ts: &[u64]) -> Result<Child, Box<dyn StdError>> {
    let list: Vec<String> = ts.iter().map(u64::to_string).collect();
    let name = "cache::window::tests::writer_process";
    let child = Command::new(std::env::current_exe()?)
        .args(["--exact", name, "--test-threads", "1", "--quiet"])
        .env(WRITER, s.base())
        .env(WRITES, list.join(","))
        .stdout(Stdio::null())
        .spawn()?;
    Ok(child)
}

fn finish(child: &mut Child) -> Test {
    let status = child.wait()?;
    if status.success() {
        return Ok(());
    }
    Err(format!("the writer process failed: {status}").into())
}

/// Every query below answered from memory, equal to the store's, and
/// so are the rows `with_index` lends (V1).
fn check(handle: &Handle, truth: &mut Store) -> Test {
    let mut bounded = from(truth, START);
    bounded.until = Some(NOW - DAY);
    let recent = from(truth, NOW - 12 * HOUR);
    for q in [from(truth, START), bounded, recent] {
        assert_eq!(same(handle, truth, &q)?, Source::Cache, "{q:?}");
        let lent = lend(handle, &q)?.ok_or("declined")?;
        assert_eq!(lent.rows, truth.observations(&q)?, "{q:?}");
    }
    agrees(handle, truth)
}

/// The index lent holds exactly the window's rows, read under the same
/// lock.
fn agrees(handle: &Handle, truth: &Store) -> Test {
    let lent = lend(handle, &from(truth, START))?.ok_or("declined")?;
    let rows = lent.rows.iter().map(|o| (o.ts, o.id.clone()));
    assert!(lent.index.keys().cloned().eq(rows), "{:?}", lent.index);
    Ok(())
}

/// Out of ts order, across buckets, into a new one and before the window.
const ROUNDS: [&[u64]; 4] = [
    &[NOW - HOUR],
    &[NOW - 3 * HOUR, NOW - 10 * DAY],
    &[NOW - 2 * DAY, START + HOUR, START - DAY],
    &[NOW - HOUR / 2],
];

/// 200 ts spread over the window, out of order.
fn burst() -> Vec<u64> {
    let step = |i: u64| i.wrapping_mul(11_059_000).wrapping_rem(27 * DAY);
    (0..200).map(|i| NOW.saturating_sub(step(i))).collect()
}

#[test]
fn cached_equals_uncached_with_a_writer_process() -> Test {
    let s = Scratch::new("v1-process")?;
    let cold = recorded(&s, &s.config()?, 1)?;
    cold.handle.warm()?;
    let mut truth = s.open(Mode::Read)?;
    for round in ROUNDS {
        finish(&mut writer(&s, round)?)?;
        check(&cold.handle, &mut truth)?;
    }
    concurrent(&s, &cold.handle, &mut truth)?;
    assert_eq!(cold.handle.query(&from(&truth, START))?.len(), 206);
    Ok(())
}

/// Query while the writer process bursts; equal once it is done.
fn concurrent(s: &Scratch, handle: &Handle, truth: &mut Store) -> Test {
    let mut busy = writer(s, &burst())?;
    while busy.try_wait()?.is_none() {
        handle.query(&from(truth, START))?;
        agrees(handle, truth)?;
    }
    finish(&mut busy)?;
    check(handle, truth)
}
