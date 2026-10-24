//! Partition files: 1 connection each, opened on first use and kept
//! (src/store C, root R5), and the rows they hold.

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use rusqlite::{Connection, params};

use super::Error;
use super::schema;
use super::write::Row;

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
