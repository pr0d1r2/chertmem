//! The per-partition finding store (src/mine C, T4): `mine/findings.db`
//! under the data dir, owned by `src/mine` alone, never `catalog.db` nor
//! a partition file (`src/store`'s). A sealed partition's findings are
//! kept under its path and project with what they were mined from: its
//! checksum, the threshold and the mining version. Any of those changed,
//! they are stale and the partition is mined again (V1).

#[cfg(test)]
mod tests;

use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, params};

use super::{Error, Finding, Kind};
use crate::store::{Partition, Ref};

/// The finding store's path under the data dir.
const FILE: &str = "mine/findings.db";

/// What mined the kept findings: bumped with every signal change, so
/// findings of an older miner are stale (V1).
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "/signals-2");

/// The lock timeout of a reading open, as the store's (src/store R2).
const BUSY: Duration = Duration::from_secs(1);

/// 1 `mined` row per partition and project mined, its findings in
/// `findings` by `ord`; refs blank-separated, without `chert:`.
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS mined (
  path TEXT NOT NULL, project TEXT NOT NULL, checksum TEXT NOT NULL,
  threshold INTEGER NOT NULL, version TEXT NOT NULL,
  PRIMARY KEY (path, project));
CREATE TABLE IF NOT EXISTS findings (
  path TEXT NOT NULL, project TEXT NOT NULL, ord INTEGER NOT NULL,
  id TEXT NOT NULL, kind TEXT NOT NULL, refs TEXT NOT NULL,
  summary TEXT NOT NULL,
  PRIMARY KEY (path, project, ord));";

const FRESH: &str = "SELECT count(*) FROM mined WHERE path = ?1
  AND project = ?2 AND checksum = ?3 AND threshold = ?4 AND version = ?5";
const KEPT: &str = "SELECT id, kind, refs, summary FROM findings
  WHERE path = ?1 AND project = ?2 ORDER BY ord";
const FORGET: &str = "DELETE FROM findings WHERE path = ?1 AND project = ?2";
const KEEP: &str = "INSERT INTO findings
  (path, project, ord, id, kind, refs, summary)
  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";
const MINED: &str = "INSERT OR REPLACE INTO mined
  (path, project, checksum, threshold, version)
  VALUES (?1, ?2, ?3, ?4, ?5)";

/// What a partition's findings were mined from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Key {
    pub(super) path: String,
    pub(super) project: String,
    /// The partition's content hash (src/store R1): its rows changed,
    /// so does this.
    pub(super) checksum: String,
    /// `mining.min_occurrences`.
    pub(super) threshold: u32,
}

impl Key {
    /// The key of `partition`'s rows of `project` at `threshold`.
    pub(super) fn of(
        partition: &Partition,
        project: &str,
        threshold: u32,
    ) -> Self {
        Self {
            path: partition.path.clone(),
            project: project.to_owned(),
            checksum: partition.stats.checksum.clone(),
            threshold,
        }
    }
}

/// 1 kept finding as stored.
struct Row {
    id: String,
    kind: String,
    refs: String,
    summary: String,
}

/// An open finding store.
#[derive(Debug)]
pub(super) struct Ledger {
    path: PathBuf,
    conn: Connection,
}

impl Ledger {
    /// The finding store under `data_dir`, laid on first use.
    pub(super) fn open(data_dir: &Path) -> Result<Self, Error> {
        let path = data_dir.join(FILE);
        let dir = path.parent().unwrap_or(data_dir);
        std::fs::create_dir_all(dir).map_err(|e| Error::ledger(dir, e))?;
        let conn = lay(&path).map_err(|e| Error::ledger(&path, e))?;
        Ok(Self { path, conn })
    }

    /// The findings kept for `key`; `None` when none are, or they are
    /// stale.
    pub(super) fn get(&self, key: &Key) -> Result<Option<Vec<Finding>>, Error> {
        if !self.fresh(key).map_err(|e| self.fail(e))? {
            return Ok(None);
        }
        let rows = self.kept(key).map_err(|e| self.fail(e))?;
        let found = rows.into_iter().map(|r| self.decode(r));
        found.collect::<Result<_, _>>().map(Some)
    }

    /// Keep `found` for `key`, in place of whatever its partition had.
    pub(super) fn put(
        &mut self,
        key: &Key,
        found: &[Finding],
    ) -> Result<(), Error> {
        let path = &self.path;
        write(&mut self.conn, key, found).map_err(|e| Error::ledger(path, e))
    }

    fn fail(&self, reason: impl Display) -> Error {
        Error::ledger(&self.path, reason)
    }

    /// Whether `key`'s partition was mined from what `key` names.
    fn fresh(&self, key: &Key) -> rusqlite::Result<bool> {
        let (k, v) = (key, VERSION);
        let args = params![k.path, k.project, k.checksum, k.threshold, v];
        let n: i64 = self.conn.query_row(FRESH, args, |r| r.get(0))?;
        Ok(n > 0)
    }

    fn kept(&self, key: &Key) -> rusqlite::Result<Vec<Row>> {
        let mut stmt = self.conn.prepare(KEPT)?;
        let rows = stmt.query_map(params![key.path, key.project], |r| {
            let (id, kind) = (r.get(0)?, r.get(1)?);
            let (refs, summary) = (r.get(2)?, r.get(3)?);
            Ok(Row {
                id,
                kind,
                refs,
                summary,
            })
        })?;
        rows.collect()
    }

    fn decode(&self, row: Row) -> Result<Finding, Error> {
        let kind = Kind::parse(&row.kind);
        let kind =
            kind.ok_or_else(|| self.fail(format!("kind {}", row.kind)))?;
        let each = row.refs.split(' ').filter(|r| !r.is_empty());
        let refs = each.map(|r| Ref::parse(&format!("chert:{r}")));
        let refs = refs.collect::<Result<Vec<_>, _>>();
        let refs = refs.map_err(|e| self.fail(e))?;
        let (id, summary, count) = (row.id, row.summary, refs.len());
        Ok(Finding {
            id,
            kind,
            count,
            refs,
            summary,
        })
    }
}

/// The finding store at `path`, WAL, its tables laid once.
fn lay(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(BUSY)?;
    let _: String =
        conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
    conn.execute_batch(SCHEMA)?;
    Ok(conn)
}

/// `found` in place of `key`'s partition's findings, in 1 transaction.
fn write(
    conn: &mut Connection,
    key: &Key,
    found: &[Finding],
) -> rusqlite::Result<()> {
    let (tx, k) = (conn.transaction()?, key);
    tx.execute(FORGET, params![k.path, k.project])?;
    for (ord, f) in (0_i64..).zip(found) {
        let refs: Vec<&str> = f.refs.iter().map(Ref::as_str).collect();
        let (kind, refs) = (f.kind.as_str(), refs.join(" "));
        let row = params![k.path, k.project, ord, f.id, kind, refs, f.summary];
        tx.execute(KEEP, row)?;
    }
    let mined = params![k.path, k.project, k.checksum, k.threshold, VERSION];
    tx.execute(MINED, mined)?;
    tx.commit()
}
