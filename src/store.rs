//! The embedded hypertable: `catalog.db` plus 1 `SQLite` file per partition
//! under the data dir (src/store G, root I).

#[cfg(test)]
mod tests;

mod calendar;
mod catalog;
mod error;
mod layout;
mod model;
mod partition;
mod reference;
mod schema;
mod write;

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

pub use error::Error;
pub use layout::project_id;
pub use model::{Appended, Event, State};
pub use reference::Ref;

use crate::config::Config;
use calendar::MAX_TS;
use layout::Plan;
use partition::Chunks;
use write::{Ctx, Row};

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
    project: String,
    plan: Plan,
    catalog: Option<Connection>,
    chunks: Chunks,
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
        let root = cfg.storage.data_dir.clone().ok_or(Error::NoDataDir)?;
        let mut store = Self {
            chunks: Chunks::new(root.clone(), mode.busy()),
            root,
            mode,
            project: project_id(project_root),
            plan: Plan::new(&cfg.partition),
            catalog: None,
        };
        if mode == Mode::Read {
            store.catalog()?;
        }
        Ok(store)
    }

    /// This store's project id ([`project_id`] of its root).
    #[must_use]
    pub fn project(&self) -> &str {
        &self.project
    }

    /// Store `event` in the partition of its own timestamp (src/store
    /// V1); an identity seen before gives back its first ref (V9).
    ///
    /// # Errors
    /// A timestamp past 48 bits, a sealed partition, an unusable file.
    pub fn append(&mut self, event: &Event) -> Result<Appended, Error> {
        if event.ts > MAX_TS {
            return Err(Error::BadTime { ts: event.ts });
        }
        let row = Row {
            id: write::new_id(event.ts),
            project: self.project.clone(),
            event: event.clone(),
        };
        let _reference = write::observation(&mut self.ctx()?, &row)?;
        Ok(Appended::Spooled) // RED stub: the ref is dropped
    }

    /// What a write reaches, the catalog opened first.
    fn ctx(&mut self) -> Result<Ctx<'_>, Error> {
        self.catalog()?;
        let catalog = self.catalog.as_ref().ok_or(Error::NoDataDir)?;
        Ok(Ctx {
            catalog,
            chunks: &mut self.chunks,
            plan: &self.plan,
        })
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
