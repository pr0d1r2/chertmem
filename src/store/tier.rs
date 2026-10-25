//! Tiers: seal (hot to warm), compact (warm to cold) and drop; every
//! rewrite or drop bumps `generation` (src/store C, T6, V5, V7). Each
//! changes the partition's files first, the catalog after (V6).

#[cfg(test)]
mod tests;

use std::path::Path;

use rusqlite::{Connection, Transaction};

use super::partition::{self, Chunks};
use super::write::immediate;
use super::{Error, State, Store, catalog, cold, generation, tombstone};
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

    /// Refuse a partition whose bucket ends after `now`: rows of its
    /// bucket still arrive (src/store V13).
    fn ended(&self, path: &str, now: u64) -> Result<(), Error> {
        let end = catalog::end(&self.tx, path)?;
        if end > now {
            let path = path.to_owned();
            return Err(Error::Open { path, end });
        }
        Ok(())
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

    /// Seal the partition at `path`, whatever the time: warm, so no
    /// append lands in it again; the next one for its bucket opens a
    /// fresh partition (src/store V7, V13). A warm or cold one stays as
    /// it is. The spool is replayed first, so what waits there lands
    /// before the seal. A caller that knows the time, as `chert
    /// partitions seal` does, calls [`Store::seal_at`] (R9).
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; no such partition.
    pub fn seal(&mut self, path: &str) -> Result<(), Error> {
        self.seal_at(path, u64::MAX)
    }

    /// [`Store::seal`] at `now` (ms since the epoch): a bucket ending
    /// after `now` is refused, its rows still arriving (src/store V13).
    ///
    /// # Errors
    /// As [`Store::seal`]; [`Error::Open`] while the bucket is open.
    pub fn seal_at(&mut self, path: &str, now: u64) -> Result<(), Error> {
        self.replay()?;
        let mut w = self.work()?;
        let state = w.state(path)?;
        w.ended(path, now)?;
        if state == Some(State::Hot) {
            w.mark(path, State::Warm)?;
        }
        Ok(w.tx.commit()?)
    }

    /// Compact the partition at `path`, whatever the time, sealing it
    /// first: `VACUUM INTO` a copy, zstd it to `<path>.zst`, drop the
    /// `.db`. Its rows, and so its stats, stay as they were.
    ///
    /// # Errors
    /// As [`Store::seal`].
    pub fn compact(&mut self, path: &str) -> Result<(), Error> {
        self.compact_at(path, u64::MAX)
    }

    /// [`Store::compact`] at `now` (ms since the epoch), refused as
    /// [`Store::seal_at`] is.
    ///
    /// # Errors
    /// As [`Store::seal_at`].
    pub fn compact_at(&mut self, path: &str, now: u64) -> Result<(), Error> {
        self.seal_at(path, now)?;
        let mut w = self.work()?;
        if w.state(path)? == Some(State::Warm) {
            w.freeze(path)?;
            catalog::set_state(&w.tx, path, State::Cold)?;
            generation::bump(&w.tx, w.root)?;
        }
        Ok(w.tx.commit()?)
    }

    /// Drop the partition at `path`: a tombstone lands beside it, its
    /// files go, its catalog row and refs stay marked, so each ref
    /// answers [`super::Lookup::Dropped`], never another row (root V5),
    /// even after a lost catalog (V12), and no write lands there again:
    /// the next one for its bucket opens a fresh partition (V13).
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; no such partition.
    pub fn drop(&mut self, path: &str) -> Result<(), Error> {
        let w = self.work()?;
        if w.state(path)?.is_some() {
            tombstone::write(&w.tx, w.root, path)?;
            w.chunks.close(path);
            cold::remove(w.root, path)?;
            catalog::mark_dropped(&w.tx, path)?;
            generation::bump(&w.tx, w.root)?;
        }
        Ok(w.tx.commit()?)
    }
}
