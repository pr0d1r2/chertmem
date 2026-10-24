//! FTS5 candidates for `src/search` (src/store T17).

#[cfg(test)]
mod tests;

use super::{Error, Filter, Frequencies, Store, Tailed};

impl Store {
    /// The rows of the partition at `path` matching any of `terms`.
    ///
    /// # Errors
    /// Not built yet.
    pub fn matches(
        &mut self,
        path: &str,
        terms: &[String],
        filter: &Filter,
    ) -> Result<Vec<Tailed>, Error> {
        let _ = (&self.root, path, terms, filter);
        Ok(Vec::new())
    }

    /// The index's counts for `terms` in the partition at `path`.
    ///
    /// # Errors
    /// Not built yet.
    pub fn frequencies(
        &mut self,
        path: &str,
        terms: &[String],
    ) -> Result<Frequencies, Error> {
        let _ = (&self.root, path, terms);
        Ok(Frequencies::default())
    }
}
