use std::error::Error as StdError;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::{Found, pending, read};
use crate::store::line::encode;
use crate::store::tests::{Scratch, Test, chunk, event, stored};
use crate::store::write::{Item, Row};
use crate::store::{Appended, Error, Mode};

fn row(identity: Option<&str>, body: &str) -> Item {
    Item::Observation(Row {
        id: "01JB2X3Y4Z5A6B7C8D9E0F1G2H".into(),
        project: "proj-0123".into(),
        event: event(1_709_211_600_000, identity, body),
    })
}

/// The catalog's write lock, held until the connection drops.
fn lock(s: &Scratch) -> Result<Connection, Box<dyn StdError>> {
    let conn = s.sql("catalog.db")?;
    conn.execute_batch("BEGIN IMMEDIATE")?;
    Ok(conn)
}

fn count(s: &Scratch, path: &str) -> Result<i64, Box<dyn StdError>> {
    let sql = "SELECT count(*) FROM observations";
    Ok(s.sql(path)?.query_row(sql, [], |r| r.get(0))?)
}

fn spooled(s: &Scratch) -> Result<Vec<PathBuf>, Box<dyn StdError>> {
    Ok(pending(&s.data())?)
}

#[test]
fn read_tells_gone_and_unreadable_apart() -> Test {
    let s = Scratch::new("spool-read")?;
    fs::create_dir_all(s.data())?;
    let path = s.data().join("x.jsonl");
    assert_eq!(read(&path)?, Found::Gone);
    for text in ["", "garbage\n", "\u{0}\u{ff}"] {
        fs::write(&path, text)?;
        assert_eq!(read(&path)?, Found::Unreadable, "{text:?}");
    }
    fs::write(&path, [0xff, 0xfe])?;
    assert_eq!(read(&path)?, Found::Unreadable);
    Ok(())
}

#[test]
fn a_held_lock_spools_and_append_still_succeeds() -> Test {
    let s = Scratch::new("spool-busy")?;
    drop(s.open(Mode::Read)?);
    let held = lock(&s)?;
    let mut hook = s.open(Mode::Write)?;
    assert_eq!(hook.append(&event(5, None, "a"))?, Appended::Spooled);
    drop(held);
    assert_eq!(spooled(&s)?.len(), 1);
    Ok(())
}

#[test]
fn a_write_only_open_never_replays() -> Test {
    let s = Scratch::new("spool-write-only")?;
    drop(s.open(Mode::Read)?);
    let held = lock(&s)?;
    s.open(Mode::Write)?.append(&event(5, None, "a"))?;
    drop(held);
    let mut hook = s.open(Mode::Write)?;
    stored(hook.append(&event(6, None, "b"))?)?;
    assert_eq!(spooled(&s)?.len(), 1);
    assert_eq!(hook.replay(), Err(Error::WriteOnly));
    Ok(())
}

#[test]
fn a_reading_open_replays_the_whole_spool_first() -> Test {
    let s = Scratch::new("spool-replay")?;
    drop(s.open(Mode::Read)?);
    let held = lock(&s)?;
    let mut hook = s.open(Mode::Write)?;
    for ts in [5, 6, 7] {
        assert_eq!(hook.append(&event(ts, None, "a"))?, Appended::Spooled);
    }
    drop(held);
    let reader = s.open(Mode::Read)?;
    assert!(spooled(&s)?.is_empty());
    assert_eq!(count(&s, &chunk(&reader, "19700101-7d"))?, 3);
    Ok(())
}

/// Spool 1 event with `identity` under a held lock; its spool file.
fn spool_one(
    s: &Scratch,
    identity: &str,
) -> Result<PathBuf, Box<dyn StdError>> {
    drop(s.open(Mode::Read)?);
    let held = lock(s)?;
    s.open(Mode::Write)?
        .append(&event(5, Some(identity), "a"))?;
    drop(held);
    spooled(s)?.pop().ok_or_else(|| "nothing spooled".into())
}

#[test]
fn replaying_a_file_twice_stores_its_row_once() -> Test {
    let s = Scratch::new("spool-twice")?;
    let file = spool_one(&s, "toolu_9")?;
    let copy = fs::read(&file)?;
    let mut reader = s.open(Mode::Read)?;
    fs::write(&file, copy)?;
    reader.replay()?;
    assert_eq!(count(&s, &chunk(&reader, "19700101-7d"))?, 1);
    let again = reader.append(&event(5, Some("toolu_9"), "a"))?;
    assert!(matches!(again, Appended::Stored(_)));
    assert_eq!(count(&s, &chunk(&reader, "19700101-7d"))?, 1);
    Ok(())
}

#[test]
fn a_lock_during_replay_keeps_the_file() -> Test {
    let s = Scratch::new("spool-replay-busy")?;
    let file = spool_one(&s, "toolu_3")?;
    let mut reader = s.open(Mode::Read)?;
    fs::write(&file, encode(&row(None, "late"))?)?;
    let held = lock(&s)?;
    let e = reader.replay().err();
    assert!(matches!(e, Some(Error::Busy { .. })), "{e:?}");
    drop(held);
    assert!(file.exists());
    Ok(())
}

#[test]
fn an_unreadable_file_is_set_aside_not_lost() -> Test {
    let s = Scratch::new("spool-aside")?;
    let spool = s.data().join("spool");
    fs::create_dir_all(&spool)?;
    fs::write(spool.join("01BAD.jsonl"), "{not a line")?;
    fs::write(spool.join(".01HALF.tmp"), "{\"type\":")?;
    drop(s.open(Mode::Read)?);
    assert!(spooled(&s)?.is_empty());
    let kept = fs::read_to_string(spool.join("unreadable/01BAD.jsonl"))?;
    assert_eq!(kept, "{not a line");
    assert!(spool.join(".01HALF.tmp").exists());
    Ok(())
}

/// `writers` threads, each its own write-only store, `each` appends.
fn hammer(
    s: &Scratch,
    writers: u64,
    each: u64,
) -> Result<(), Box<dyn StdError>> {
    let (cfg, root) = (s.config()?, s.project());
    let run = |w: u64| -> Result<(), crate::store::Error> {
        let mut store = crate::store::Store::open(&cfg, &root, Mode::Write)?;
        let base = w.saturating_mul(1000);
        for ts in base..base.saturating_add(each) {
            store.append(&event(ts, None, "x"))?;
        }
        Ok(())
    };
    std::thread::scope(|scope| {
        let handles: Vec<_> =
            (0..writers).map(|w| scope.spawn(move || run(w))).collect();
        handles.into_iter().try_for_each(joined)
    })
}

fn joined(
    h: std::thread::ScopedJoinHandle<'_, Result<(), crate::store::Error>>,
) -> Result<(), Box<dyn StdError>> {
    match h.join() {
        Ok(r) => r.map_err(Into::into),
        Err(_) => Err("a writer panicked".into()),
    }
}

#[test]
fn concurrent_writers_lose_no_event() -> Test {
    let s = Scratch::new("spool-hammer")?;
    drop(s.open(Mode::Read)?);
    hammer(&s, 6, 40)?;
    let reader = s.open(Mode::Read)?;
    assert!(spooled(&s)?.is_empty());
    assert_eq!(count(&s, &chunk(&reader, "19700101-7d"))?, 240);
    Ok(())
}

#[test]
fn no_spool_dir_is_an_empty_spool() -> Test {
    assert!(pending(Path::new("/no/such/chert/data"))?.is_empty());
    Ok(())
}
