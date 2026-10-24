//! `generation`: never the same value twice, across a lost catalog and
//! `rebuild` too, so a cache holding an old one never reads stale as
//! fresh (src/store V11, R4). Each bump moves it to the largest of the
//! stored value + 1, the clock in ms and the mark + 1; the mark is the
//! file `generation` beside `catalog.db`, written on every bump.

#[cfg(test)]
mod tests;

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use super::Error;
use super::write::immediate;

const BUMP: &str = "UPDATE meta SET value = max(value + 1, ?1)
  WHERE key = 'generation' RETURNING value";
const VALUE: &str = "SELECT value FROM meta WHERE key = 'generation'";

/// The mark: the last value any catalog in this data dir gave out.
fn mark(root: &Path) -> PathBuf {
    root.join("generation")
}

/// Milliseconds since the epoch.
fn now() -> i64 {
    let since = SystemTime::now().duration_since(UNIX_EPOCH);
    i64::try_from(since.unwrap_or_default().as_millis()).unwrap_or(i64::MAX)
}

/// The mark's value; 0 when there is none yet, or it does not parse.
pub(super) fn marked(root: &Path) -> Result<i64, Error> {
    let path = mark(root);
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(text.trim().parse().unwrap_or(0)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(0),
        Err(e) => Err(Error::io(&path, &e)),
    }
}

/// Bump `generation` (D15) under the caller's write lock, the mark
/// written before the caller commits: a mark ahead of a rolled-back
/// catalog only raises the floor.
pub(super) fn bump(conn: &Connection, root: &Path) -> Result<(), Error> {
    let floor = now().max(marked(root)?.saturating_add(1));
    let value: i64 = conn.query_row(BUMP, [floor], |r| r.get(0))?;
    let path = mark(root);
    let name = ulid::Ulid::generate();
    let tmp = root.join(format!(".generation.{name}.tmp"));
    std::fs::write(&tmp, format!("{value}\n"))
        .map_err(|e| Error::io(&tmp, &e))?;
    std::fs::rename(&tmp, &path).map_err(|e| Error::io(&path, &e))
}

/// A new catalog, its `generation` still 0, starts past every value
/// an earlier catalog in this data dir gave out.
pub(super) fn seed(conn: &Connection, root: &Path) -> Result<(), Error> {
    let value: i64 = conn.query_row(VALUE, [], |r| r.get(0))?;
    if value != 0 {
        return Ok(());
    }
    let tx = immediate(conn)?;
    bump(&tx, root)?;
    Ok(tx.commit()?)
}
