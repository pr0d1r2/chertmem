//! The read side of [`Store`]: every call needs a reading open, which
//! replayed the spool first (src/store V10), and routes by the catalog
//! only (V4).

#[cfg(test)]
mod tests;

use rusqlite::Connection;

use super::catalog::{self, Hit};
use super::{
    Error, Filter, Lookup, Observation, Partition, Ref, Store, partition, prune,
};

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

    /// Resolve each ref by exact match on the stored ref (root V15), in
    /// order: [`Lookup::Found`] with its row, or [`Lookup::Dropped`]
    /// when the row or its partition is gone, never another row (V5).
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; an unreadable file.
    pub fn get(&mut self, refs: &[Ref]) -> Result<Vec<Lookup>, Error> {
        refs.iter().map(|r| self.resolve(r)).collect()
    }

    /// This project first, then every project; issued in more than 1
    /// other project is an error naming them, never a guess (root V15).
    fn resolve(&mut self, reference: &Ref) -> Result<Lookup, Error> {
        let (catalog, text) = (self.reader()?, reference.as_str());
        let mut hits = catalog::locate(catalog, Some(&self.project), text)?;
        if hits.is_empty() {
            hits = catalog::locate(catalog, None, text)?;
        }
        match hits.as_slice() {
            [] => Ok(Lookup::Dropped(reference.clone())),
            [hit] => self.fetch(hit, reference),
            many => Err(Error::Ambiguous {
                reference: reference.to_string(),
                projects: many.iter().map(|h| h.project.clone()).collect(),
            }),
        }
    }

    /// The row behind `hit`; a missing file or row is [`Lookup::Dropped`].
    fn fetch(&mut self, hit: &Hit, reference: &Ref) -> Result<Lookup, Error> {
        let dropped = Lookup::Dropped(reference.clone());
        let Some(conn) = self.chunks.existing(&hit.path)? else {
            return Ok(dropped);
        };
        let row = partition::fetch(conn, &hit.project, reference.as_str())?;
        Ok(row.map_or(dropped, Lookup::Found))
    }

    /// The catalog's `generation`: bumped on every delete, redaction
    /// or rewrite, so a cache knows its rows went stale (D15).
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; a failed query.
    pub fn generation(&self) -> Result<u64, Error> {
        let sql = "SELECT value FROM meta WHERE key = 'generation'";
        Ok(self.reader()?.query_row(sql, [], |r| r.get(0))?)
    }

    /// The catalog of a reading open (src/store V10).
    fn reader(&self) -> Result<&Connection, Error> {
        self.reading()?;
        self.catalog.as_ref().ok_or(Error::WriteOnly)
    }
}
