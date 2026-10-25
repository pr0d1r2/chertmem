use std::error::Error as StdError;

use rusqlite::Connection;

use crate::store::tests::{Scratch, Test, WEEK, event, stored};
use crate::store::{
    Error, Filter, Mode, Session, SessionStart, Store, Written,
};

fn start(id: &str, ts: u64) -> SessionStart {
    SessionStart {
        id: id.into(),
        ide: "claude-code".into(),
        cwd: format!("/work/{id}"),
        ts,
    }
}

fn session(id: &str, started_at: u64, ended_at: Option<u64>) -> Session {
    Session {
        id: id.into(),
        ide: "claude-code".into(),
        cwd: format!("/work/{id}"),
        started_at,
        ended_at,
    }
}

/// Record `id` started at `t0` and, when given, ended at `t1`.
fn record(store: &mut Store, id: &str, t0: u64, t1: Option<u64>) -> Test {
    assert_eq!(store.session_started(&start(id, t0))?, Written::Stored);
    if let Some(t1) = t1 {
        assert_eq!(store.session_ended(id, t1)?, Written::Stored);
    }
    Ok(())
}

#[test]
fn a_session_spanning_two_buckets_is_1_row() -> Test {
    let s = Scratch::new("session-span")?;
    let (t0, t1) = (WEEK + 5, 3 * WEEK + 7);
    let mut hook = s.open(Mode::Write)?;
    record(&mut hook, "s1", t0, None)?;
    stored(hook.append(&event(t0, None, "a"))?)?;
    stored(hook.append(&event(t1, None, "b"))?)?;
    assert_eq!(hook.session_ended("s1", t1)?, Written::Stored);
    let mut store = s.open(Mode::Read)?;
    assert_eq!(store.partitions(&Filter::default())?.len(), 2);
    assert_eq!(store.sessions(10)?, [session("s1", t0, Some(t1))]);
    Ok(())
}

#[test]
fn starts_and_ends_are_idempotent() -> Test {
    let s = Scratch::new("session-idem")?;
    let mut store = s.open(Mode::Read)?;
    for _ in 0..2 {
        record(&mut store, "s1", 100, Some(200))?;
    }
    assert_eq!(store.sessions(10)?, [session("s1", 100, Some(200))]);
    record(&mut store, "s1", 150, Some(190))?;
    assert_eq!(store.sessions(10)?, [session("s1", 100, Some(200))]);
    record(&mut store, "s1", 90, Some(300))?;
    assert_eq!(store.sessions(10)?, [session("s1", 90, Some(300))]);
    Ok(())
}

#[test]
fn newest_first_this_project_only_up_to_limit() -> Test {
    let s = Scratch::new("session-list")?;
    let mut other = s.open_other("other")?;
    record(&mut other, "elsewhere", 900, None)?;
    let mut store = s.open(Mode::Read)?;
    record(&mut store, "old", 100, Some(150))?;
    record(&mut store, "new", 300, None)?;
    record(&mut store, "mid", 200, Some(250))?;
    let newest = [session("new", 300, None), session("mid", 200, Some(250))];
    assert_eq!(store.sessions(2)?, newest);
    assert_eq!(store.sessions(10)?.len(), 3);
    assert_eq!(store.sessions(0)?, []);
    Ok(())
}

#[test]
fn an_end_without_a_start_lists_until_the_start_arrives() -> Test {
    let s = Scratch::new("session-end-first")?;
    let mut store = s.open(Mode::Read)?;
    assert_eq!(store.session_ended("s1", 500)?, Written::Stored);
    let bare = Session {
        ide: String::new(),
        cwd: String::new(),
        ..session("s1", 500, Some(500))
    };
    assert_eq!(store.sessions(10)?, [bare]);
    record(&mut store, "s1", 400, None)?;
    assert_eq!(store.sessions(10)?, [session("s1", 400, Some(500))]);
    Ok(())
}

#[test]
fn a_write_only_store_records_but_never_lists() -> Test {
    let s = Scratch::new("session-write")?;
    let mut hook = s.open(Mode::Write)?;
    record(&mut hook, "s1", 100, Some(200))?;
    assert_eq!(hook.sessions(10).err(), Some(Error::WriteOnly));
    assert_eq!(s.open(Mode::Read)?.sessions(10)?.len(), 1);
    Ok(())
}

/// The catalog's write lock, held until the connection drops.
fn lock(s: &Scratch) -> Result<Connection, Box<dyn StdError>> {
    let conn = s.sql("catalog.db")?;
    conn.execute_batch("BEGIN IMMEDIATE")?;
    Ok(conn)
}

#[test]
fn a_held_lock_spools_both_and_a_reading_open_replays_them() -> Test {
    let s = Scratch::new("session-spool")?;
    drop(s.open(Mode::Read)?);
    let held = lock(&s)?;
    let mut hook = s.open(Mode::Write)?;
    assert_eq!(hook.session_started(&start("s1", 100))?, Written::Spooled);
    assert_eq!(hook.session_ended("s1", 200)?, Written::Spooled);
    drop(held);
    let mut store = s.open(Mode::Read)?;
    assert_eq!(store.sessions(10)?, [session("s1", 100, Some(200))]);
    Ok(())
}
