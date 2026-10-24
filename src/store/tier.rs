//! Tiers: seal (hot to warm), compact (warm to cold) and drop; every
//! rewrite or drop bumps `generation` (src/store C, T6, V5, V7).

#[cfg(test)]
mod tests;

use super::{Error, Store, cold};

impl Store {
    /// Seal the partition at `path`: warm, so every append into it is
    /// refused with [`Error::ReadOnly`] (src/store V7).
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; no such partition.
    pub fn seal(&mut self, path: &str) -> Result<(), Error> {
        let _ = path;
        self.reading() // RED stub
    }

    /// Compact the partition at `path`, sealing it first: `VACUUM INTO`
    /// a copy, zstd it to `<path>.zst`, drop the `.db`.
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; no such partition.
    pub fn compact(&mut self, path: &str) -> Result<(), Error> {
        self.reading()?;
        let src = cold::zst(&self.root, path);
        cold::freeze(&src, &self.root, path, self.zstd) // RED stub
    }

    /// Drop the partition at `path`: its files go, its refs answer
    /// [`super::Lookup::Dropped`], never another row (root V5).
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; no such partition.
    pub fn drop(&mut self, path: &str) -> Result<(), Error> {
        self.reading()?;
        cold::remove(&self.root, path) // RED stub
    }

    /// Rewrite every partition written under another `partition.*`
    /// spec to the current one; refs keep resolving (src/store V5).
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; a sealed target.
    pub fn repartition(&mut self) -> Result<(), Error> {
        self.reading() // RED stub
    }
}
