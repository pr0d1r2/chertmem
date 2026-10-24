//! Tiers: seal (hot to warm), compact (warm to cold) and drop; every
//! rewrite or drop bumps `generation` (src/store C, T6, V5, V7). Each
//! changes the partition's files first, the catalog after (V6).

#[cfg(test)]
mod tests;

use std::path::Path;

use rusqlite::{Connection, Transaction};

use super::partition::{self, Chunks};
use super::write::immediate;
use super::{Error, State, Store, catalog, cold};
use crate::config::Zstd;

/// What a tier move reaches, under the catalog's write lock.
struct Work<'a> {
    tx: Transaction<'a>,
    chunks: &'a mut Chunks,
    root: &'a Path,
    zstd: Zstd,
}

impl Work<'_> {
    /// The tier of the partition at `path`; `None` once dropped.
    fn state(&self, path: &str) -> Result<Option<State>, Error> {
        Ok(catalog::state(&self.tx, path)?.1)
    }

    /// Record `state` in the file, then in the catalog.
    fn mark(&mut self, path: &str, state: State) -> Result<(), Error> {
        if let Some(conn) = self.chunks.existing(path)? {
            partition::set_state(conn, state)?;
        }
        catalog::set_state(&self.tx, path, state)
    }

    /// `VACUUM INTO` a copy stamped cold, zstd it, drop the hot file.
    fn freeze(&mut self, path: &str) -> Result<(), Error> {
        let copy = self.root.join(format!("{path}.vacuum"));
        cold::unlink(&copy)?;
        let gone = || Error::Io {
            path: self.root.join(path),
            reason: "no such file".to_owned(),
        };
        let conn = self.chunks.existing(path)?.ok_or_else(gone)?;
        conn.execute("VACUUM INTO ?1", [copy.to_string_lossy()])?;
        partition::set_state(&Connection::open(&copy)?, State::Cold)?;
        cold::freeze(&copy, self.root, path, self.zstd)?;
        cold::unlink(&copy)?;
        self.chunks.close(path);
        cold::remove_hot(self.root, path)
    }
}

impl Store {
    /// The catalog's write lock and what a tier move reaches; a
    /// write-only store refuses (src/store V10).
    fn work(&mut self) -> Result<Work<'_>, Error> {
        self.reading()?;
        let catalog = self.catalog.as_ref().ok_or(Error::WriteOnly)?;
        Ok(Work {
            tx: immediate(catalog)?,
            chunks: &mut self.chunks,
            root: &self.root,
            zstd: self.zstd,
        })
    }

    /// Seal the partition at `path`: warm, so every append into it is
    /// refused with [`Error::ReadOnly`] (src/store V7). A warm or cold
    /// one stays as it is. The spool is replayed first, so what waits
    /// there lands before the seal; what a hook spools during it is held
    /// in `spool/sealed/` (T10).
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; no such partition.
    pub fn seal(&mut self, path: &str) -> Result<(), Error> {
        self.replay()?;
        let mut w = self.work()?;
        if w.state(path)? == Some(State::Hot) {
            w.mark(path, State::Warm)?;
        }
        Ok(w.tx.commit()?)
    }

    /// Compact the partition at `path`, sealing it first: `VACUUM INTO`
    /// a copy, zstd it to `<path>.zst`, drop the `.db`.
    /// Its rows, and so its stats, stay as they were.
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; no such partition.
    pub fn compact(&mut self, path: &str) -> Result<(), Error> {
        self.seal(path)?;
        let mut w = self.work()?;
        if w.state(path)? == Some(State::Warm) {
            w.freeze(path)?;
            catalog::set_state(&w.tx, path, State::Cold)?;
            catalog::bump(&w.tx)?;
        }
        Ok(w.tx.commit()?)
    }

    /// Drop the partition at `path`: its files go, its catalog row and
    /// refs stay marked, so each ref answers [`super::Lookup::Dropped`],
    /// never another row (root V5), and no write lands there again.
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; no such partition.
    pub fn drop(&mut self, path: &str) -> Result<(), Error> {
        let w = self.work()?;
        if w.state(path)?.is_some() {
            w.chunks.close(path);
            cold::remove(w.root, path)?;
            catalog::mark_dropped(&w.tx, path)?;
            catalog::bump(&w.tx)?;
        }
        Ok(w.tx.commit()?)
    }
}
