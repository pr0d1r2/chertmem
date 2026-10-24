//! Partition files: 1 connection each, opened on first use and kept
//! (src/store C, root R5), and the rows they hold.

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, params};

use super::schema;
use super::write::Row;
use super::{Error, Filter, Observation, Ref};

/// The open partition files of 1 store.
#[derive(Debug)]
pub(super) struct Chunks {
    root: PathBuf,
    busy: Duration,
    open: BTreeMap<String, Connection>,
}

impl Chunks {
    pub(super) fn new(root: PathBuf, busy: Duration) -> Self {
        Self {
            root,
            busy,
            open: BTreeMap::new(),
        }
    }

    /// The partition at `path` (relative to the data dir), made when new.
    pub(super) fn get(&mut self, path: &str) -> Result<&Connection, Error> {
        if !self.open.contains_key(path) {
            let file = self.root.join(path);
            if let Some(dir) = file.parent() {
                std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, &e))?;
            }
            let conn = schema::open(&file, self.busy, schema::PARTITION)?;
            self.open.insert(path.to_owned(), conn);
        }
        self.open.get(path).ok_or(Error::NoDataDir)
    }

    /// The partition at `path` when its file exists; a read never makes
    /// one, so a dropped file reads as gone.
    pub(super) fn existing(
        &mut self,
        path: &str,
    ) -> Result<Option<&Connection>, Error> {
        if !self.open.contains_key(path) && !self.root.join(path).is_file() {
            return Ok(None);
        }
        self.get(path).map(Some)
    }
}

const SCAN: &str = "SELECT ref, id, project, ts, session, ide, kind,
  identity, body FROM observations
  WHERE (?1 IS NULL OR project = ?1)
    AND (?2 IS NULL OR ts >= ?2)
    AND (?3 IS NULL OR ts < ?3)
  ORDER BY ts, id";

/// The rows of 1 partition `filter` wants, by time then id.
pub(super) fn scan(
    conn: &Connection,
    filter: &Filter,
) -> Result<Vec<Observation>, Error> {
    let mut stmt = conn.prepare(SCAN)?;
    let args = params![filter.project, filter.since, filter.until];
    let rows = stmt.query_map(args, observation)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

const FETCH: &str = "SELECT ref, id, project, ts, session, ide, kind,
  identity, body FROM observations WHERE project = ?1 AND ref = ?2";

/// The row stored under exactly `reference` in `project`.
pub(super) fn fetch(
    conn: &Connection,
    project: &str,
    reference: &str,
) -> Result<Option<Observation>, Error> {
    let args = [project, reference];
    Ok(conn.query_row(FETCH, args, observation).optional()?)
}

/// An observation from the columns `ref, id, project, ts, session, ide,
/// kind, identity, body`.
pub(super) fn observation(
    r: &rusqlite::Row<'_>,
) -> rusqlite::Result<Observation> {
    Ok(Observation {
        reference: Ref::stored(r.get(0)?),
        id: r.get(1)?,
        project: r.get(2)?,
        ts: r.get(3)?,
        session: r.get(4)?,
        ide: r.get(5)?,
        kind: r.get(6)?,
        identity: r.get(7)?,
        body: r.get(8)?,
    })
}

const INSERT: &str = "INSERT INTO observations
  (id, ref, project, identity, ts, session, ide, kind, body)
  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
  ON CONFLICT DO NOTHING";

const STORED: &str = "SELECT ref FROM observations
  WHERE id = ?1 OR (project = ?2 AND identity = ?3)";

/// Insert `row` under `issued`; the ref the partition holds for it, the
/// earlier one when a crash left the row here but not in the catalog.
pub(super) fn insert(
    conn: &Connection,
    row: &Row,
    issued: &str,
) -> Result<String, Error> {
    let (e, id, p) = (&row.event, &row.id, &row.project);
    let args = params![
        id, issued, p, e.identity, e.ts, e.session, e.ide, e.kind, e.body
    ];
    conn.execute(INSERT, args)?;
    let key = params![id, p, e.identity];
    Ok(conn.query_row(STORED, key, |r| r.get(0))?)
}
