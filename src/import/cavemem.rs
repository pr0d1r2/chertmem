//! cavemem 0.2.1 import: its `SQLite` file opened read-only (`mode=ro`),
//! never written (src/import C, V3); each row stored once however often
//! it is imported (V2).
//!
//! Mapping: an observation's `session_id`, `kind`, `ts` and its
//! session's `ide` go over as they are; `content` is redacted again and,
//! when cavemem left it uncompressed, compressed; the identity is
//! `cavemem:<session_id>:<id>`, so a 2nd import finds the 1st row
//! (src/store V9). The session's `cwd` picks the project (`split`).
//! `intensity`, `metadata`, the session's `started_at`, `ended_at`,
//! `metadata`, the FTS index and `embeddings` are dropped: chert keeps
//! none of them.

#[cfg(test)]
mod tests;

mod split;
mod summary;

use std::path::Path;

use rusqlite::{Connection, OpenFlags, OptionalExtension, Row};

use super::Error;
use crate::config::{Config, Grammar};
use crate::grammar::{self, Intensity};
use crate::store::Event;
use split::Cwd;

/// What an import read from cavemem; every row of it is in the store.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Imported {
    pub observations: usize,
    pub summaries: usize,
    /// Observations and summaries whose session has no `cwd` (missing
    /// or empty), stored under the current project (src/import T5).
    pub no_cwd: usize,
}

/// The `schema_version` cavemem 0.2.1 writes.
const VERSION: i64 = 2;

const OBSERVATIONS: &str = "SELECT o.id, o.session_id, o.kind, o.content,
    o.compressed, o.ts, COALESCE(s.ide, ''), s.cwd
  FROM observations o LEFT JOIN sessions s ON s.id = o.session_id
  ORDER BY o.id";

/// Import the cavemem 0.2.1 file `db` into the data dir `config` names,
/// each session's rows under the project of its own `cwd`, as the hook
/// derives it ([`crate::store::project_id`]); a session without one
/// goes to the project at `root`. A body cavemem stored uncompressed is
/// compressed at `compression.grammar`, as the hook does.
///
/// # Errors
/// A file that is not a cavemem 0.2.1 db, or a store that refuses a
/// read or a write.
pub fn cavemem(
    config: &Config,
    root: &Path,
    db: &Path,
) -> Result<Imported, Error> {
    let (conn, level) = (open(db)?, intensity(config.compression.grammar));
    let events = observations(&conn, level)?;
    let notes = summary::read(&conn, level)?;
    drop(conn);
    split::put(config, root, events, notes)
}

/// `db` read-only, never created, its schema version checked.
fn open(db: &Path) -> Result<Connection, Error> {
    let uri = format!("file:{}?mode=ro", escaped(db));
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY
        | OpenFlags::SQLITE_OPEN_URI
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(uri, flags)?;
    let sql = "SELECT 1 FROM schema_version WHERE version = ?1";
    let found: Option<i64> =
        conn.query_row(sql, [VERSION], |r| r.get(0)).optional()?;
    found.ok_or_else(|| Error::Cavemem {
        reason: format!("no schema_version {VERSION}"),
    })?;
    Ok(conn)
}

/// `path` as a URI path: the characters a URI gives meaning escaped.
fn escaped(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.replace('%', "%25")
        .replace('?', "%3f")
        .replace('#', "%23")
}

/// Every observation as the event chert stores, in cavemem's order.
fn observations(
    conn: &Connection,
    intensity: Intensity,
) -> Result<Vec<(Cwd, Event)>, Error> {
    let mut stmt = conn.prepare(OBSERVATIONS)?;
    let rows = stmt.query_map([], raw)?;
    let found = rows.collect::<Result<Vec<_>, _>>()?;
    found.into_iter().map(|r| event(r, intensity)).collect()
}

/// 1 observation row as cavemem stored it.
struct Raw {
    id: i64,
    session: String,
    kind: String,
    content: String,
    compressed: bool,
    ts: i64,
    ide: String,
    cwd: Option<String>,
}

fn raw(r: &Row<'_>) -> rusqlite::Result<Raw> {
    Ok(Raw {
        id: r.get(0)?,
        session: r.get(1)?,
        kind: r.get(2)?,
        content: r.get(3)?,
        compressed: r.get(4)?,
        ts: r.get(5)?,
        ide: r.get(6)?,
        cwd: r.get(7)?,
    })
}

/// 1 row as the event chert stores, with its session's cwd.
fn event(r: Raw, intensity: Intensity) -> Result<(Cwd, Event), Error> {
    let event = Event {
        ts: millis(r.ts)?,
        identity: Some(format!("cavemem:{}:{}", r.session, r.id)),
        body: body(&r.content, r.compressed, intensity),
        session: r.session,
        kind: r.kind,
        ide: r.ide,
    };
    Ok((split::cwd(r.cwd), event))
}

/// cavemem's `Date.now()` milliseconds; before the epoch is refused.
fn millis(ts: i64) -> Result<u64, Error> {
    u64::try_from(ts).map_err(|_| Error::Cavemem {
        reason: format!("timestamp {ts} is before 1970"),
    })
}

/// `content` as chert stores it: private spans removed first (root V8),
/// then compressed unless cavemem already did.
fn body(content: &str, compressed: bool, intensity: Intensity) -> String {
    let kept = crate::hook::redact::strip(content);
    if compressed {
        kept
    } else {
        grammar::compress(&kept, intensity)
    }
}

fn intensity(grammar: Grammar) -> Intensity {
    match grammar {
        Grammar::Off => Intensity::Off,
        Grammar::Lite => Intensity::Lite,
        Grammar::Full => Intensity::Full,
        Grammar::Ultra => Intensity::Ultra,
    }
}
