//! The embedded hypertable: `catalog.db` plus 1 `SQLite` file per partition
//! under the data dir (src/store G, root I).

#[cfg(test)]
mod tests;

mod error;
mod schema;

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

pub use error::Error;

use crate::config::Config;

/// How a process uses the store (src/store V10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// A hook: appends only, a short lock timeout, never replays the
    /// spool, so its latency does not grow with the spool.
    Write,
    /// Every other caller: replays the whole spool before its first read.
    Read,
}

impl Mode {
    /// The lock timeout before a write goes to the spool (src/store R2).
    const fn busy(self) -> Duration {
        match self {
            Self::Write => Duration::from_millis(2),
            Self::Read => Duration::from_secs(1),
        }
    }
}

/// An open store; 1 connection per partition, never `ATTACH` (root R5).
#[derive(Debug)]
pub struct Store {
    root: PathBuf,
    mode: Mode,
    catalog: Option<Connection>,
}

impl Store {
    /// Open the data dir `cfg` names for the project at `project_root`.
    /// A [`Mode::Read`] open lays the catalog now; a [`Mode::Write`] open
    /// touches nothing until its first append.
    ///
    /// # Errors
    /// No data dir, an unusable file, or a lock still held after the busy
    /// timeout.
    pub fn open(
        cfg: &Config,
        project_root: &Path,
        mode: Mode,
    ) -> Result<Self, Error> {
        let _ = project_root;
        let root = cfg.storage.data_dir.clone().ok_or(Error::NoDataDir)?;
        let mut store = Self {
            root,
            mode,
            catalog: None,
        };
        if mode == Mode::Read {
            store.catalog()?;
        }
        Ok(store)
    }

    /// The catalog connection, laid on first use with the data dir.
    fn catalog(&mut self) -> Result<&Connection, Error> {
        if self.catalog.is_none() {
            for dir in ["chunks", "spool"] {
                let path = self.root.join(dir);
                std::fs::create_dir_all(&path)
                    .map_err(|e| Error::io(&path, &e))?;
            }
            let path = self.root.join("catalog.db");
            let conn = schema::open(&path, self.mode.busy(), schema::CATALOG)?;
            self.catalog = Some(conn);
        }
        self.catalog.as_ref().ok_or(Error::NoDataDir)
    }
}
