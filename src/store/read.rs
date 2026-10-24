//! The read side of [`Store`]: every call needs a reading open, which
//! replayed the spool first (src/store V10), and routes by the catalog
//! only (V4).

#[cfg(test)]
mod tests;

use rusqlite::Connection;

use super::{Error, Filter, Observation, Partition, Store, partition, prune};

impl Store {
    /// The catalog rows of every partition that may hold a row `filter`
    /// wants, by `t0` then path (src/store V4).
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; a failed query.
    pub fn partitions(&self, filter: &Filter) -> Result<Vec<Partition>, Error> {
        prune::partitions(self.reader()?, filter)
    }

    /// Every observation `filter` wants, across the partitions the
    /// catalog names, by time then id. A partition whose file is gone
    /// holds nothing.
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; an unreadable file.
    pub fn observations(
        &mut self,
        filter: &Filter,
    ) -> Result<Vec<Observation>, Error> {
        let mut found = Vec::new();
        for p in self.partitions(filter)? {
            if let Some(conn) = self.chunks.existing(&p.path)? {
                found.extend(partition::scan(conn, filter)?);
            }
        }
        found.sort_by(|a, b| (a.ts, &a.id).cmp(&(b.ts, &b.id)));
        Ok(found)
    }

    /// The catalog of a reading open (src/store V10).
    fn reader(&self) -> Result<&Connection, Error> {
        self.reading()?;
        self.catalog.as_ref().ok_or(Error::WriteOnly)
    }
}
