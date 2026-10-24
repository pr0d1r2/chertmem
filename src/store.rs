//! The embedded hypertable: `catalog.db` plus 1 `SQLite` file per partition
//! under the data dir (src/store G, root I).

#[cfg(test)]
mod tests;

mod calendar;
mod catalog;
mod cold;
mod doctor;
mod error;
mod generation;
mod id;
mod layout;
mod line;
mod model;
mod partition;
mod prune;
mod read;
mod reference;
mod repartition;
mod restore;
mod schema;
mod session;
mod spool;
mod summary;
mod tail;
mod tier;
mod tombstone;
mod write;

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

pub use error::Error;
pub use layout::project_id;
pub use model::{
    Appended, Event, Filter, Lookup, Observation, Partition, Scope, Session,
    SessionStart, State, Stats, Summary, Tailed, Written,
};
pub use reference::Ref;

use crate::config::{Config, Zstd};
use calendar::MAX_TS;
use layout::Plan;
use partition::Chunks;
use write::{Ctx, Item, Note, Row};

/// A new id at `ts`, which a ULID must hold (src/store T15).
fn new_id(ts: u64) -> Result<String, Error> {
    if ts > MAX_TS {
        return Err(Error::BadTime { ts });
    }
    id::new(ts)
}

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
    zstd: Zstd,
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
            zstd: cfg.compression.zstd,
        };
        if mode == Mode::Read {
            store.catalog()?;
            store.replay()?;
        }
        Ok(store)
    }

    /// Replay the whole spool into its partitions, oldest first; a
    /// reading open does this before its first read (src/store V10).
    /// Each file is dropped once replayed; replaying it twice, as 2
    /// readers at once may, stores its row once (its id is known).
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; a lock outlasting the
    /// busy timeout leaves the rest of the spool for the next replay.
    pub fn replay(&mut self) -> Result<(), Error> {
        self.reading()?;
        for path in spool::pending(&self.root)? {
            self.replay_file(&path)?;
        }
        Ok(())
    }

    /// Replay 1 spool file, then drop it; an unreadable one is set
    /// aside, and one for a sealed partition held (src/store T10).
    fn replay_file(&mut self, path: &Path) -> Result<(), Error> {
        let items = match spool::read(path)? {
            spool::Found::Items(items) => items,
            spool::Found::Unreadable => {
                return spool::set_aside(&self.root, path);
            }
            spool::Found::Gone => return Ok(()),
        };
        match self.apply_all(&items) {
            Err(Error::ReadOnly { .. }) => spool::hold(&self.root, path),
            done => done.and_then(|()| spool::done(path)),
        }
    }

    /// Store every replayed item in order.
    fn apply_all(&mut self, items: &[Item]) -> Result<(), Error> {
        for item in items {
            write::apply(&mut self.ctx()?, item)?;
        }
        Ok(())
    }

    /// Refuse a read on a write-only store (src/store V10).
    fn reading(&self) -> Result<(), Error> {
        match self.mode {
            Mode::Read => Ok(()),
            Mode::Write => Err(Error::WriteOnly),
        }
    }

    /// This store's project id ([`project_id`] of its root).
    #[must_use]
    pub fn project(&self) -> &str {
        &self.project
    }

    /// Keep a hook payload that could not be parsed, `bytes` as given,
    /// in `spool/unreadable/<ULID>.hook` under the data dir, written to
    /// a temp file and renamed into place: no replay reads, stores or
    /// drops it, so it waits for a person or an importer (src/store
    /// T14). Allowed in both modes; the file kept is returned.
    ///
    /// # Errors
    /// [`Error::Io`] when the dir or the file cannot be written.
    pub fn spool_raw(&self, bytes: &[u8]) -> Result<PathBuf, Error> {
        spool::raw(&self.root, bytes)
    }

    /// Store `event` in the partition of its own timestamp (src/store
    /// V1); an identity seen before gives back its first ref (V9). A
    /// lock outlasting the busy timeout spools it instead and still
    /// succeeds (V10).
    ///
    /// # Errors
    /// A timestamp past 48 bits, a sealed partition, an unusable file.
    pub fn append(&mut self, event: &Event) -> Result<Appended, Error> {
        let row = Row {
            id: new_id(event.ts)?,
            project: self.project.clone(),
            event: event.clone(),
        };
        let item = Item::Observation(row.clone());
        let stored = self.store(&item, |cx| write::observation(cx, &row))?;
        Ok(stored.map_or(Appended::Spooled, Appended::Stored))
    }

    /// Store a turn or session summary in the partition of its own
    /// timestamp (src/store T9); spooled like an event when a lock
    /// outlasts the busy timeout (V10).
    ///
    /// # Errors
    /// A timestamp past 48 bits, a sealed partition, an unusable file.
    pub fn append_summary(
        &mut self,
        summary: &Summary,
    ) -> Result<Written, Error> {
        let note = Note {
            id: new_id(summary.ts)?,
            project: self.project.clone(),
            summary: summary.clone(),
        };
        let item = Item::Summary(note.clone());
        let stored = self.store(&item, |cx| write::summary(cx, &note))?;
        Ok(stored.map_or(Written::Spooled, |()| Written::Stored))
    }

    /// Run `write` under the catalog lock; a lock outlasting the busy
    /// timeout spools `item` instead and answers `None` (V10).
    fn store<T>(
        &mut self,
        item: &Item,
        write: impl FnOnce(&mut Ctx<'_>) -> Result<T, Error>,
    ) -> Result<Option<T>, Error> {
        match self.ctx().and_then(|mut cx| write(&mut cx)) {
            Ok(done) => Ok(Some(done)),
            Err(Error::Busy { .. }) => {
                spool::put(&self.root, item).map(|()| None)
            }
            Err(e) => Err(e),
        }
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

    /// The catalog connection, laid on first use with the data dir; a
    /// new catalog's `generation` is seeded past every earlier one (V11).
    fn catalog(&mut self) -> Result<&Connection, Error> {
        if self.catalog.is_none() {
            for dir in ["chunks", "spool"] {
                let path = self.root.join(dir);
                std::fs::create_dir_all(&path)
                    .map_err(|e| Error::io(&path, &e))?;
            }
            let path = self.root.join("catalog.db");
            let conn = schema::open(&path, self.mode.busy(), schema::CATALOG)?;
            generation::seed(&conn, &self.root)?;
            self.catalog = Some(conn);
        }
        self.catalog.as_ref().ok_or(Error::NoDataDir)
    }
}
