//! The per-partition finding store (src/mine C, T4): `mine/findings.db`
//! under the data dir, owned by `src/mine` alone, never `catalog.db` nor
//! a partition file (`src/store`'s). A sealed partition's findings are
//! kept under its path and project with what they were mined from: its
//! checksum, the threshold and the mining version. Any of those changed,
//! they are stale and the partition is mined again (V1).

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use super::{Error, Finding};
use crate::store::Partition;

/// The finding store's path under the data dir.
const FILE: &str = "mine/findings.db";

/// What a partition's findings were mined from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Key {
    pub(super) path: String,
    pub(super) project: String,
    /// The partition's content hash (src/store R1): its rows changed,
    /// so does this.
    pub(super) checksum: String,
    /// `mining.min_occurrences`.
    pub(super) threshold: u32,
}

impl Key {
    /// The key of `partition`'s rows of `project` at `threshold`.
    pub(super) fn of(
        partition: &Partition,
        project: &str,
        threshold: u32,
    ) -> Self {
        Self {
            path: partition.path.clone(),
            project: project.to_owned(),
            checksum: partition.stats.checksum.clone(),
            threshold,
        }
    }
}

/// An open finding store.
#[derive(Debug)]
pub(super) struct Ledger {
    path: PathBuf,
}

impl Ledger {
    /// The finding store under `data_dir`, laid on first use.
    pub(super) fn open(data_dir: &Path) -> Result<Self, Error> {
        let path = data_dir.join(FILE);
        let dir = path.parent().unwrap_or(data_dir);
        std::fs::create_dir_all(dir).map_err(|e| Error::ledger(dir, e))?;
        Ok(Self { path })
    }

    /// The findings kept for `key`; `None` when none are, or they are
    /// stale.
    #[expect(clippy::unnecessary_wraps, reason = "RED stub")]
    pub(super) fn get(&self, key: &Key) -> Result<Option<Vec<Finding>>, Error> {
        let _ = (&self.path, &key.path, &key.project, &key.checksum);
        let _ = key.threshold;
        Ok(None)
    }

    /// Keep `found` for `key`, in place of whatever its partition had.
    #[expect(
        clippy::unnecessary_wraps,
        clippy::unused_self,
        clippy::needless_pass_by_ref_mut,
        reason = "RED stub"
    )]
    pub(super) fn put(
        &mut self,
        key: &Key,
        found: &[Finding],
    ) -> Result<(), Error> {
        let _ = (key, found);
        Ok(())
    }
}
