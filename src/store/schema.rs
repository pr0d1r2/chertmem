//! The SQL of `catalog.db` and of every partition file, and how each file
//! is opened: WAL, a busy timeout, the schema laid once (src/store C, T1).

#[cfg(test)]
mod tests;

use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, Transaction, TransactionBehavior};

use super::Error;

/// `PRAGMA user_version` once a file holds its schema.
const VERSION: i64 = 1;

/// The catalog: 1 row per partition (src/store C), the `generation`
/// counter (D15) and every issued ref with its event identity, so a ref
/// or an identity is unique per project across partitions (root V15,
/// V16).
pub(super) const CATALOG: &str = "
CREATE TABLE meta (key TEXT PRIMARY KEY, value INTEGER NOT NULL);
INSERT INTO meta (key, value) VALUES ('generation', 0);
CREATE TABLE partitions (
  id INTEGER PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  dims TEXT NOT NULL,
  project TEXT NOT NULL,
  ide TEXT NOT NULL,
  t0 INTEGER NOT NULL,
  t1 INTEGER NOT NULL,
  shard INTEGER,
  rows INTEGER NOT NULL DEFAULT 0,
  bytes INTEGER NOT NULL DEFAULT 0,
  state TEXT NOT NULL DEFAULT 'hot'
    CHECK (state IN ('hot', 'warm', 'cold')),
  checksum TEXT NOT NULL,
  spec TEXT NOT NULL,
  dropped INTEGER NOT NULL DEFAULT 0 CHECK (dropped IN (0, 1)),
  CHECK (t0 < t1));
CREATE INDEX partitions_bounds ON partitions (t0, t1);
CREATE TABLE refs (
  project TEXT NOT NULL,
  ref TEXT NOT NULL,
  id TEXT NOT NULL UNIQUE,
  identity TEXT,
  partition INTEGER NOT NULL REFERENCES partitions (id),
  PRIMARY KEY (project, ref),
  UNIQUE (project, identity));
CREATE INDEX refs_ref ON refs (ref);
";

/// A partition: its own catalog row's dims, bounds, spec and tier in
/// `chunk`, so `doctor` can rebuild the catalog from files (src/store
/// V6); observations with their FTS5 external-content index kept by
/// triggers, and summaries (src/store C).
pub(super) const PARTITION: &str = "
CREATE TABLE chunk (
  one INTEGER PRIMARY KEY CHECK (one = 1),
  dims TEXT NOT NULL,
  project TEXT NOT NULL,
  ide TEXT NOT NULL,
  t0 INTEGER NOT NULL,
  t1 INTEGER NOT NULL,
  shard INTEGER,
  spec TEXT NOT NULL,
  state TEXT NOT NULL DEFAULT 'hot'
    CHECK (state IN ('hot', 'warm', 'cold')));
CREATE TABLE observations (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  id TEXT NOT NULL UNIQUE,
  ref TEXT NOT NULL,
  project TEXT NOT NULL,
  identity TEXT,
  ts INTEGER NOT NULL,
  session TEXT NOT NULL,
  ide TEXT NOT NULL,
  kind TEXT NOT NULL,
  body TEXT NOT NULL,
  UNIQUE (project, ref),
  UNIQUE (project, identity));
CREATE INDEX observations_session ON observations (session, ts);
CREATE INDEX observations_ts ON observations (ts);
CREATE VIRTUAL TABLE observations_fts USING fts5 (
  body, content = 'observations', content_rowid = 'seq');
CREATE TRIGGER observations_ai AFTER INSERT ON observations BEGIN
  INSERT INTO observations_fts (rowid, body) VALUES (new.seq, new.body);
END;
CREATE TRIGGER observations_ad AFTER DELETE ON observations BEGIN
  INSERT INTO observations_fts (observations_fts, rowid, body)
    VALUES ('delete', old.seq, old.body);
END;
CREATE TRIGGER observations_au AFTER UPDATE ON observations BEGIN
  INSERT INTO observations_fts (observations_fts, rowid, body)
    VALUES ('delete', old.seq, old.body);
  INSERT INTO observations_fts (rowid, body) VALUES (new.seq, new.body);
END;
CREATE TABLE summaries (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  id TEXT NOT NULL UNIQUE,
  project TEXT NOT NULL,
  session TEXT NOT NULL,
  scope TEXT NOT NULL CHECK (scope IN ('turn', 'session')),
  ts INTEGER NOT NULL,
  ide TEXT NOT NULL,
  body TEXT NOT NULL);
CREATE INDEX summaries_session ON summaries (project, session, scope, ts);
";

/// Open `path` in WAL mode with `busy` as its lock timeout and lay
/// `schema` unless the file already holds it.
pub(super) fn open(
    path: &Path,
    busy: Duration,
    schema: &str,
) -> Result<Connection, Error> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(busy)?;
    let mode: String =
        conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(Error::Sqlite {
            reason: format!("{} stays in {mode} mode", path.display()),
        });
    }
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    lay(&conn, schema)?;
    Ok(conn)
}

/// The schema, once: checked again under the write lock, so 2 processes
/// opening a new file at once lay it once.
fn lay(conn: &Connection, schema: &str) -> Result<(), Error> {
    if version(conn)? == VERSION {
        return Ok(());
    }
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    if version(&tx)? != VERSION {
        tx.execute_batch(schema)?;
        tx.pragma_update(None, "user_version", VERSION)?;
    }
    tx.commit()?;
    Ok(())
}

fn version(conn: &Connection) -> Result<i64, Error> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}
