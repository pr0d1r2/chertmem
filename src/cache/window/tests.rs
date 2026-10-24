//! The window alone, and V1 & V2 through a handle with a real second
//! process writing.

use std::error::Error as StdError;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use super::Window;
use crate::cache::tests::{
    DAY, HOUR, NOW, START, Scratch, Test, config_at, event, from, same,
};
use crate::cache::{Handle, Source};
use crate::store::{Filter, Mode, Store};

/// A window from [`START`] with every partition loaded.
fn loaded(store: &mut Store) -> Result<Window, Box<dyn StdError>> {
    let mut window = Window::begin(store, START)?;
    for path in window.paths() {
        window.sync(store, &path)?;
    }
    Ok(window)
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
    let rows = window.answer(&q).ok_or("declined")?;
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
        assert_eq!(window.answer(&q), None, "{q:?}");
    }
    assert_eq!(window.answer(&from(&store, START)), Some(Vec::new()));
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
    window.refresh(&mut store)?;
    let q = from(&store, START);
    let rows = window.answer(&q).ok_or("declined")?;
    assert_eq!((rows.len(), window.paths().len()), (3, 2));
    assert_eq!(rows, store.observations(&q)?);
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

/// Every query below answered from memory, equal to the store's (V1).
fn check(handle: &Handle, truth: &mut Store) -> Test {
    let mut bounded = from(truth, START);
    bounded.until = Some(NOW - DAY);
    let recent = from(truth, NOW - 12 * HOUR);
    for q in [from(truth, START), bounded, recent] {
        assert_eq!(same(handle, truth, &q)?, Source::Cache, "{q:?}");
    }
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
    let cold = s.cold(&s.config()?)?;
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
    }
    finish(&mut busy)?;
    check(handle, truth)
}
