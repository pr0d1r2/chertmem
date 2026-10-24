//! cavemem import tests, and the fixture db built from cavemem's own
//! schema and insert statements.

use std::fs;
use std::path::Path;

use rusqlite::{Connection, params};

use super::super::Error;
use super::super::tests::{NOW, Res, Scratch, Test};
use super::{Imported, cavemem};
use crate::config::Grammar;
use crate::grammar::{Intensity, compress};
use crate::store::{Filter, Observation, Store};

/// cavemem 0.2.1's `SCHEMA_SQL`, verbatim from
/// `packages/storage/src/schema.ts` as bundled in `npm pack
/// cavemem@0.2.1`, `dist/chunk-T35V7EPZ.js` (root R1).
const SCHEMA: &str = r"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;

CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY);

CREATE TABLE IF NOT EXISTS sessions (
  id TEXT PRIMARY KEY,
  ide TEXT NOT NULL,
  cwd TEXT,
  started_at INTEGER NOT NULL,
  ended_at INTEGER,
  metadata TEXT
);

CREATE TABLE IF NOT EXISTS observations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
  kind TEXT NOT NULL,
  content TEXT NOT NULL,
  compressed INTEGER NOT NULL DEFAULT 1,
  intensity TEXT,
  ts INTEGER NOT NULL,
  metadata TEXT
);
CREATE INDEX IF NOT EXISTS idx_observations_session ON observations(session_id, ts);
CREATE INDEX IF NOT EXISTS idx_observations_ts ON observations(ts);

CREATE TABLE IF NOT EXISTS summaries (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
  scope TEXT NOT NULL CHECK(scope IN ('turn','session')),
  content TEXT NOT NULL,
  compressed INTEGER NOT NULL DEFAULT 1,
  intensity TEXT,
  ts INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_summaries_session ON summaries(session_id, ts);

CREATE VIRTUAL TABLE IF NOT EXISTS observations_fts USING fts5(
  content,
  content='observations',
  content_rowid='id',
  tokenize='porter unicode61'
);

CREATE TRIGGER IF NOT EXISTS obs_ai AFTER INSERT ON observations BEGIN
  INSERT INTO observations_fts(rowid, content) VALUES (new.id, new.content);
END;
CREATE TRIGGER IF NOT EXISTS obs_ad AFTER DELETE ON observations BEGIN
  INSERT INTO observations_fts(observations_fts, rowid, content) VALUES('delete', old.id, old.content);
END;
CREATE TRIGGER IF NOT EXISTS obs_au AFTER UPDATE ON observations BEGIN
  INSERT INTO observations_fts(observations_fts, rowid, content) VALUES('delete', old.id, old.content);
  INSERT INTO observations_fts(rowid, content) VALUES (new.id, new.content);
END;

CREATE TABLE IF NOT EXISTS embeddings (
  observation_id INTEGER PRIMARY KEY REFERENCES observations(id) ON DELETE CASCADE,
  model TEXT NOT NULL,
  dim INTEGER NOT NULL,
  vec BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_embeddings_model ON embeddings(model, dim);

INSERT OR IGNORE INTO schema_version(version) VALUES (2);
";

/// cavemem's `createSession` and `insertObservation` statements,
/// verbatim from the same bundle.
pub(super) const SESSION: &str = "INSERT OR IGNORE INTO sessions(id, ide, cwd, started_at, metadata) VALUES (?, ?, ?, ?, ?)";
pub(super) const OBSERVATION: &str = "INSERT INTO observations(session_id, kind, content, compressed, intensity, ts, metadata) VALUES (?, ?, ?, ?, ?, ?, ?)";

/// A cavemem db at `path`: 2 sessions, 3 observations (1 stored
/// uncompressed, 1 with a private span left by `redactSecrets` off).
pub(super) fn fixture(path: &Path) -> Res<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(SCHEMA)?;
    for (id, ide) in [("s-a", "claude-code"), ("s-b", "cursor")] {
        let args = params![id, ide, "/w", NOW, None::<String>];
        conn.execute(SESSION, args)?;
    }
    observations(&conn)?;
    Ok(conn)
}

/// The fixture's observations, by cavemem's own insert.
fn observations(conn: &Connection) -> Res<()> {
    let rows = [
        ("s-a", "post-tool-use", "ran ls", 1, 10),
        ("s-a", "user-prompt-submit", "Please fix the build", 0, 20),
        ("s-b", "stop", "ok <private>tok-9</private> done", 1, 30),
    ];
    for (s, kind, content, compressed, at) in rows {
        let ts = NOW.saturating_add(at);
        let args =
            params![s, kind, content, compressed, "full", ts, None::<String>];
        conn.execute(OBSERVATION, args)?;
    }
    Ok(())
}

/// Every observation in `store`.
fn rows(store: &mut Store) -> Res<Vec<Observation>> {
    Ok(store.observations(&Filter::default())?)
}

/// Import the fixture at `s`'s `cavemem.db` (built on first call).
fn imported(s: &Scratch, store: &mut Store) -> Res<Imported> {
    let db = s.path("cavemem.db");
    if !db.exists() {
        fixture(&db)?;
    }
    Ok(cavemem(store, &db, Grammar::Full)?)
}

/// The ts, session, ide and kind of `o`.
fn fields(o: &Observation) -> (u64, &str, &str, &str) {
    (o.ts, &o.session, &o.ide, &o.kind)
}

#[test]
fn every_observation_lands_with_its_session_and_ide() -> Test {
    let s = Scratch::new("cm-rows")?;
    let mut store = s.store()?;
    assert_eq!(imported(&s, &mut store)?.observations, 3);
    let got = rows(&mut store)?;
    let seen: Vec<_> = got.iter().map(fields).collect();
    let at = |n: u64| NOW.saturating_add(n);
    let want = [
        (at(10), "s-a", "claude-code", "post-tool-use"),
        (at(20), "s-a", "claude-code", "user-prompt-submit"),
        (at(30), "s-b", "cursor", "stop"),
    ];
    assert_eq!(seen, want);
    Ok(())
}

#[test]
fn bodies_are_redacted_and_compressed_as_the_hook_would() -> Test {
    let s = Scratch::new("cm-bodies")?;
    let mut store = s.store()?;
    imported(&s, &mut store)?;
    let bodies: Vec<String> =
        rows(&mut store)?.into_iter().map(|o| o.body).collect();
    let prompt = compress("Please fix the build", Intensity::Full);
    assert_ne!(prompt, "Please fix the build");
    assert_eq!(bodies, ["ran ls".to_owned(), prompt, "ok  done".to_owned()]);
    Ok(())
}

#[test]
fn each_row_carries_its_cavemem_identity() -> Test {
    let s = Scratch::new("cm-identity")?;
    let mut store = s.store()?;
    imported(&s, &mut store)?;
    let ids: Vec<Option<String>> =
        rows(&mut store)?.into_iter().map(|o| o.identity).collect();
    let want = ["cavemem:s-a:1", "cavemem:s-a:2", "cavemem:s-b:3"];
    assert_eq!(ids, want.map(|w| Some(w.to_owned())));
    Ok(())
}

#[test]
fn importing_twice_keeps_the_same_rows() -> Test {
    let s = Scratch::new("cm-twice")?;
    let mut store = s.store()?;
    imported(&s, &mut store)?;
    let first = rows(&mut store)?;
    assert_eq!(imported(&s, &mut store)?.observations, 3);
    assert_eq!(rows(&mut store)?, first);
    assert_eq!(first.len(), 3);
    Ok(())
}

#[test]
fn the_cavemem_file_is_byte_identical_after_import() -> Test {
    let s = Scratch::new("cm-bytes")?;
    let db = s.path("cavemem.db");
    drop(fixture(&db)?);
    let before = fs::read(&db)?;
    let mut store = s.store()?;
    imported(&s, &mut store)?;
    imported(&s, &mut store)?;
    assert_eq!(fs::read(&db)?, before);
    Ok(())
}

/// The error importing `db` gives.
fn refused(s: &Scratch, db: &Path) -> Res<Error> {
    let got = cavemem(&mut s.store()?, db, Grammar::Full);
    got.err().ok_or_else(|| "import succeeded".into())
}

#[test]
fn a_missing_file_is_refused_and_not_made() -> Test {
    let s = Scratch::new("cm-missing")?;
    let missing = s.path("none.db");
    assert!(matches!(refused(&s, &missing)?, Error::Cavemem { .. }));
    assert!(!missing.exists());
    Ok(())
}

#[test]
fn a_foreign_file_is_refused() -> Test {
    let s = Scratch::new("cm-foreign")?;
    let text = s.path("notes.db");
    fs::write(&text, "not sqlite at all, just text")?;
    assert!(matches!(refused(&s, &text)?, Error::Cavemem { .. }));
    let other = s.path("other.db");
    Connection::open(&other)?.execute_batch("CREATE TABLE t (x);")?;
    assert!(matches!(refused(&s, &other)?, Error::Cavemem { .. }));
    Ok(())
}

#[test]
fn another_schema_version_is_refused() -> Test {
    let s = Scratch::new("cm-version")?;
    let db = s.path("cavemem.db");
    let conn = fixture(&db)?;
    conn.execute_batch(
        "DELETE FROM schema_version; INSERT INTO schema_version VALUES (3);",
    )?;
    drop(conn);
    let e = refused(&s, &db)?;
    assert!(e.to_string().contains("schema_version"), "{e}");
    Ok(())
}

#[test]
fn a_negative_timestamp_is_refused() -> Test {
    let s = Scratch::new("cm-negative")?;
    let db = s.path("cavemem.db");
    let conn = fixture(&db)?;
    conn.execute("UPDATE observations SET ts = -5 WHERE id = 1", [])?;
    drop(conn);
    let e = refused(&s, &db)?;
    assert!(e.to_string().contains("-5"), "{e}");
    Ok(())
}

#[test]
fn a_path_with_uri_characters_opens() -> Test {
    let s = Scratch::new("cm-uri")?;
    let db = s.path("cave %20?#mem.db");
    drop(fixture(&db)?);
    let got = cavemem(&mut s.store()?, &db, Grammar::Full)?;
    assert_eq!(got.observations, 3);
    Ok(())
}
