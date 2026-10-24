//! What `src/cache` reads to stay fresh: `PRAGMA data_version` of the
//! catalog and of a partition, and a partition's rows by seq, never by
//! ULID order (`src/cache` V2, V3).

#[cfg(test)]
mod tests;

use rusqlite::{Connection, params};

use super::partition::observation;
use super::{Error, Filter, Store, Tailed};

const TAIL: &str = "SELECT ref, id, project, ts, session, ide, kind,
  identity, body, seq FROM observations
  WHERE seq > ?1
    AND (?2 IS NULL OR project = ?2)
    AND (?3 IS NULL OR ts >= ?3)
    AND (?4 IS NULL OR ts < ?4)
  ORDER BY seq";

fn version(conn: &Connection) -> Result<u64, Error> {
    Ok(conn.query_row("PRAGMA data_version", [], |r| r.get(0))?)
}

/// The rows of 1 partition past `after` that `filter` wants, by seq.
pub(super) fn rows(
    conn: &Connection,
    after: i64,
    filter: &Filter,
) -> Result<Vec<Tailed>, Error> {
    let mut stmt = conn.prepare(TAIL)?;
    let f = filter;
    let args = params![after, f.project, f.since, f.until];
    let found = stmt.query_map(args, |r| {
        Ok(Tailed {
            seq: r.get(9)?,
            observation: observation(r)?,
        })
    })?;
    Ok(found.collect::<Result<_, _>>()?)
}

impl Store {
    /// `PRAGMA data_version` of the catalog connection: it moves when
    /// another connection commits to the catalog.
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; a failed query.
    pub fn catalog_version(&self) -> Result<u64, Error> {
        version(self.reader()?)
    }

    /// `PRAGMA data_version` of the partition at `path` (as in
    /// [`super::Partition::path`]); `None` when its file is gone.
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; an unreadable file.
    pub fn data_version(&mut self, path: &str) -> Result<Option<u64>, Error> {
        self.reading()?;
        self.chunks.existing(path)?.map(version).transpose()
    }

    /// The rows of the partition at `path` with seq past `after` that
    /// `filter` wants, by seq; `after = 0` loads the whole partition. A
    /// gone file holds nothing.
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; an unreadable file.
    pub fn tail(
        &mut self,
        path: &str,
        after: i64,
        filter: &Filter,
    ) -> Result<Vec<Tailed>, Error> {
        self.reading()?;
        match self.chunks.existing(path)? {
            Some(conn) => rows(conn, after, filter),
            None => Ok(Vec::new()),
        }
    }
}
