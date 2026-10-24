use std::error::Error as StdError;
use std::fs;
use std::time::Duration;

use rusqlite::Connection;

use super::{VALUE, bump, marked, seed};
use crate::store::Error;
use crate::store::schema::{self, CATALOG};
use crate::store::tests::{Scratch, Test};

/// A fresh catalog under the data dir, `generation` still 0.
fn catalog(s: &Scratch) -> Result<Connection, Box<dyn StdError>> {
    fs::create_dir_all(s.data())?;
    let path = s.data().join("catalog.db");
    Ok(schema::open(&path, Duration::from_secs(1), CATALOG)?)
}

fn value(conn: &Connection) -> Result<i64, Box<dyn StdError>> {
    Ok(conn.query_row(VALUE, [], |r| r.get(0))?)
}

#[test]
fn a_bump_goes_past_the_mark_and_writes_it() -> Test {
    let s = Scratch::new("generation-mark")?;
    let conn = catalog(&s)?;
    fs::write(s.data().join("generation"), "9000000000000\n")?;
    bump(&conn, &s.data())?;
    assert_eq!(value(&conn)?, 9_000_000_000_001);
    assert_eq!(marked(&s.data())?, 9_000_000_000_001);
    Ok(())
}

#[test]
fn a_bump_goes_past_the_clock() -> Test {
    let s = Scratch::new("generation-clock")?;
    let conn = catalog(&s)?;
    fs::write(s.data().join("generation"), "not a number")?;
    assert_eq!(marked(&s.data())?, 0);
    bump(&conn, &s.data())?;
    assert!(value(&conn)? > 1_700_000_000_000);
    Ok(())
}

#[test]
fn a_mark_that_is_a_dir_is_an_io_error() -> Test {
    let s = Scratch::new("generation-dir")?;
    let conn = catalog(&s)?;
    fs::create_dir_all(s.data().join("generation"))?;
    let e = bump(&conn, &s.data()).err();
    assert!(matches!(e, Some(Error::Io { .. })), "{e:?}");
    Ok(())
}

#[test]
fn seed_moves_only_a_new_catalog() -> Test {
    let s = Scratch::new("generation-seed")?;
    let conn = catalog(&s)?;
    seed(&conn, &s.data())?;
    let seeded = value(&conn)?;
    assert!(seeded > 0);
    seed(&conn, &s.data())?;
    assert_eq!(value(&conn)?, seeded);
    Ok(())
}
