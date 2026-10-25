//! The heartbeat `chert statusline` reads (src/mcp T4, R4): while it
//! runs, a server holds an exclusive lock on
//! `<data>/mcp/<project-id>.<pid>.lock` and, after each reply is
//! flushed, writes the median latency of its last 20 replies to
//! `<data>/mcp/<project-id>.<pid>.json` (temp file + rename). A clean
//! exit removes both; a lock nobody holds is a server that is gone.

#[cfg(test)]
mod tests;

use std::collections::VecDeque;
use std::fs::{self, File, TryLockError};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Value, json};

/// The replies the median is taken over.
const WINDOW: usize = 20;

/// A running server's heartbeat files, removed on drop.
#[derive(Debug)]
pub(super) struct Beat {
    /// Held for the server's life: its lock says it runs.
    _lock: File,
    /// `<data>/mcp`.
    dir: PathBuf,
    /// `<project-id>.<pid>`: each file is this plus `.<ext>`.
    stem: String,
    pid: u32,
    /// The last [`WINDOW`] latencies in microseconds, oldest first.
    recent: VecDeque<u64>,
}

impl Beat {
    /// Lock this server's file under `<data_dir>/mcp`.
    ///
    /// # Errors
    /// The dir or file cannot be made, or the lock is held already.
    pub(super) fn start(
        data_dir: &Path,
        project: &str,
        pid: u32,
    ) -> io::Result<Self> {
        let dir = data_dir.join("mcp");
        fs::create_dir_all(&dir)?;
        let stem = format!("{project}.{pid}");
        let lock = File::create(dir.join(format!("{stem}.lock")))?;
        lock.try_lock().map_err(io::Error::from)?;
        Ok(Self {
            _lock: lock,
            dir,
            stem,
            pid,
            recent: VecDeque::with_capacity(WINDOW),
        })
    }

    /// Note 1 reply that `took` and write the beat at `now` (ms): 1
    /// small file written and renamed, no fsync, so its cost stays
    /// bounded whatever the session's length.
    ///
    /// # Errors
    /// The beat file cannot be written.
    pub(super) fn record(
        &mut self,
        took: Duration,
        now: u64,
    ) -> io::Result<()> {
        if self.recent.len() == WINDOW {
            self.recent.pop_front();
        }
        let us = u64::try_from(took.as_micros()).unwrap_or(u64::MAX);
        self.recent.push_back(us);
        let beat = json!({"pid": self.pid, "median_us": self.median(),
            "replies": self.recent.len(), "at": now});
        let tmp = self.path("json.tmp");
        fs::write(&tmp, beat.to_string())?;
        fs::rename(&tmp, self.path("json"))
    }

    /// The upper median: of an even count, the higher middle one, so a
    /// published latency errs slow.
    fn median(&self) -> u64 {
        let mut sorted: Vec<u64> = self.recent.iter().copied().collect();
        sorted.sort_unstable();
        sorted.get(sorted.len() / 2).copied().unwrap_or(0)
    }

    fn path(&self, ext: &str) -> PathBuf {
        self.dir.join(format!("{}.{ext}", self.stem))
    }
}

impl Drop for Beat {
    fn drop(&mut self) {
        for ext in ["json", "json.tmp", "lock"] {
            let _ = fs::remove_file(self.path(ext));
        }
    }
}

/// Whether any `chert mcp` of a project runs, as its lock files say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pulse {
    /// No server holds a lock.
    Dead,
    /// The freshest beat of the servers holding one; `None` before the
    /// first reply or when it cannot be read.
    Live { median_us: Option<u64> },
}

/// The pulse of `project`'s servers under `data_dir`; never an error,
/// as a badge must not fail. Reads only `<data_dir>/mcp`.
#[must_use]
pub fn pulse(data_dir: &Path, project: &str) -> Pulse {
    let Ok(entries) = fs::read_dir(data_dir.join("mcp")) else {
        return Pulse::Dead;
    };
    let prefix = format!("{project}.");
    let locks = entries.filter_map(|e| Some(e.ok()?.path()));
    let mine = locks.filter(|p| is_lock(p, &prefix));
    let beats: Vec<Option<(u64, u64)>> =
        mine.filter(|p| held(p)).map(|p| beat(&p)).collect();
    if beats.is_empty() {
        return Pulse::Dead;
    }
    let freshest = beats.into_iter().flatten().max_by_key(|&(at, _)| at);
    Pulse::Live {
        median_us: freshest.map(|(_, us)| us),
    }
}

/// `path` is a lock file of the project `prefix` names.
fn is_lock(path: &Path, prefix: &str) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let lock = path.extension().is_some_and(|e| e == "lock");
    lock && name.starts_with(prefix)
}

/// Some server holds the lock at `path`: this process cannot take it.
fn held(path: &Path) -> bool {
    File::open(path)
        .is_ok_and(|f| matches!(f.try_lock(), Err(TryLockError::WouldBlock)))
}

/// `(at, median_us)` of the beat beside the lock at `path`.
fn beat(path: &Path) -> Option<(u64, u64)> {
    let text = fs::read_to_string(path.with_extension("json")).ok()?;
    let beat: Value = serde_json::from_str(&text).ok()?;
    let field = |k: &str| beat.get(k).and_then(Value::as_u64);
    Some((field("at")?, field("median_us")?))
}
