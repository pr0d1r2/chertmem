//! A read-only look at the catalog for `chert statusline` (src/cli
//! T3): no dir made, no spool replayed, no partition opened, so a badge
//! drawn every turn costs 1 catalog query. Rows still in the spool are
//! not counted until a reading open replays them (V10).

#[cfg(test)]
mod tests;

use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};

use super::layout::Plan;
use super::partition::Chunks;
use super::{Error, Mode, Store, project_id};
use crate::config::Config;

/// A WAL reader rarely waits; a badge should not wait long when it does.
const BUSY: Duration = Duration::from_millis(5);

const FLAGS: OpenFlags =
    OpenFlags::SQLITE_OPEN_READ_ONLY.union(OpenFlags::SQLITE_OPEN_NO_MUTEX);

impl Store {
    /// The catalog of `cfg`'s data dir for the project at
    /// `project_root`, opened read-only; `None` without a data dir or a
    /// catalog. Only catalog reads work on it: an append fails.
    ///
    /// # Errors
    /// An unreadable catalog.
    pub fn peek(
        cfg: &Config,
        project_root: &Path,
    ) -> Result<Option<Self>, Error> {
        let root = cfg.storage.data_dir.clone();
        let Some(root) = root.filter(|r| r.join("catalog.db").is_file()) else {
            return Ok(None);
        };
        let conn = Connection::open_with_flags(root.join("catalog.db"), FLAGS)?;
        conn.busy_timeout(BUSY)?;
        Ok(Some(Self {
            chunks: Chunks::new(root.clone(), BUSY),
            root,
            mode: Mode::Read,
            project: project_id(project_root),
            plan: Plan::new(&cfg.partition),
            catalog: Some(conn),
            zstd: cfg.compression.zstd,
        }))
    }
}
