//! What `src/cache` reads to stay fresh: `PRAGMA data_version` of the
//! catalog and of a partition, and a partition's rows by seq, never by
//! ULID order (`src/cache` V2, V3).

#[cfg(test)]
mod tests;

use super::{Error, Filter, Store, Tailed};

impl Store {
    /// `PRAGMA data_version` of the catalog connection: it moves when
    /// another connection commits to the catalog.
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; a failed query.
    pub fn catalog_version(&self) -> Result<u64, Error> {
        Ok(0) // RED stub
    }

    /// `PRAGMA data_version` of the partition at `path` (as in
    /// [`super::Partition::path`]); `None` when its file is gone.
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; an unreadable file.
    pub fn data_version(&mut self, path: &str) -> Result<Option<u64>, Error> {
        let _ = path;
        Ok(None) // RED stub
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
        let _ = (path, after, filter);
        Ok(Vec::new()) // RED stub
    }
}
