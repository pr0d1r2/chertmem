use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use rusqlite::Connection;

use super::{CATALOG, PARTITION, open};

type Test = Result<(), Box<dyn Error>>;

const BUSY: Duration = Duration::from_millis(50);

/// A fresh file path in its own scratch dir.
fn scratch(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("chert-schema-{pid}-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    Ok(dir.join("file.db"))
}

fn names(conn: &Connection, kind: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type = ?1 ORDER BY name",
    )?;
    let rows = stmt.query_map([kind], |r| r.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn mode(conn: &Connection) -> Result<String, Box<dyn Error>> {
    Ok(conn.query_row("PRAGMA journal_mode", [], |r| r.get(0))?)
}

#[test]
fn catalog_holds_partitions_refs_and_generation() -> Test {
    let path = scratch("catalog")?;
    let conn = open(&path, BUSY, CATALOG)?;
    let tables = names(&conn, "table")?;
    assert_eq!(tables, ["meta", "partitions", "refs"]);
    let generation: i64 = conn.query_row(
        "SELECT value FROM meta WHERE key = 'generation'",
        [],
        |r| r.get(0),
    )?;
    assert_eq!((mode(&conn)?, generation), ("wal".to_owned(), 0));
    Ok(())
}

#[test]
fn reopening_keeps_the_schema_once() -> Test {
    let path = scratch("reopen")?;
    drop(open(&path, BUSY, CATALOG)?);
    let conn = open(&path, BUSY, CATALOG)?;
    let n: i64 =
        conn.query_row("SELECT count(*) FROM meta", [], |r| r.get(0))?;
    assert_eq!(n, 1);
    Ok(())
}

const ROW: &str = "INSERT INTO observations
  (id, ref, project, ts, session, ide, kind, body)
  VALUES (?1, ?1, 'p', 1, 's', 'claude', 'note', ?2)";

fn hits(conn: &Connection, word: &str) -> Result<i64, Box<dyn Error>> {
    Ok(conn.query_row(
        "SELECT count(*) FROM observations_fts WHERE observations_fts \
         MATCH ?1",
        [word],
        |r| r.get(0),
    )?)
}

#[test]
fn partition_indexes_bodies_in_fts5() -> Test {
    let conn = open(&scratch("fts")?, BUSY, PARTITION)?;
    conn.execute(ROW, ["A", "cargo build failed"])?;
    conn.execute(ROW, ["B", "tests pass"])?;
    assert_eq!((hits(&conn, "cargo")?, hits(&conn, "pass")?), (1, 1));
    conn.execute("UPDATE observations SET body = 'moved' WHERE id = 'B'", [])?;
    assert_eq!((hits(&conn, "pass")?, hits(&conn, "moved")?), (0, 1));
    conn.execute("DELETE FROM observations WHERE id = 'A'", [])?;
    assert_eq!(hits(&conn, "cargo")?, 0);
    Ok(())
}

#[test]
fn partition_refuses_a_second_identity_in_a_project() -> Test {
    let conn = open(&scratch("identity")?, BUSY, PARTITION)?;
    let sql = "INSERT INTO observations
      (id, ref, project, identity, ts, session, ide, kind, body)
      VALUES (?1, ?1, 'p', ?2, 1, 's', 'claude', 'note', 'b')";
    conn.execute(sql, ["A", "tool-1"])?;
    assert!(conn.execute(sql, ["B", "tool-1"]).is_err());
    conn.execute(
        "INSERT INTO observations
          (id, ref, project, ts, session, ide, kind, body)
          VALUES ('C', 'C', 'p', 1, 's', 'claude', 'note', 'b'),
                 ('D', 'D', 'p', 1, 's', 'claude', 'note', 'b')",
        [],
    )?;
    Ok(())
}

#[test]
fn summaries_take_turn_and_session_scopes_only() -> Test {
    let conn = open(&scratch("scope")?, BUSY, PARTITION)?;
    let sql = "INSERT INTO summaries (id, project, session, scope, ts, ide,
      body) VALUES (?1, 'p', 's', ?2, 1, 'claude', 'b')";
    conn.execute(sql, ["A", "turn"])?;
    conn.execute(sql, ["B", "session"])?;
    assert!(conn.execute(sql, ["C", "week"]).is_err());
    Ok(())
}

#[test]
fn an_unopenable_path_is_an_error() -> Test {
    let dir = scratch("unopenable")?;
    let missing = dir.with_file_name("no-such-dir").join("file.db");
    assert!(open(&missing, BUSY, CATALOG).is_err());
    Ok(())
}
