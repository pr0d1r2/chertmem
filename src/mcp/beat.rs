//! The heartbeat `chert statusline` reads (src/mcp T4, R4): while it
//! runs, a server holds an exclusive lock on
//! `<data>/mcp/<project-id>.<pid>.lock` and, after each reply is
//! flushed, writes the median latency of its last 20 replies to
//! `<data>/mcp/<project-id>.<pid>.json` (temp file + rename). A clean
//! exit removes both; a lock nobody holds is a server that is gone.
//! RED stub: nothing is written, every server reads dead.

#[cfg(test)]
mod tests;

use std::io;
use std::path::Path;
use std::time::Duration;

/// A running server's heartbeat files.
#[derive(Debug)]
pub(super) struct Beat {
    recent: Vec<Duration>,
}

impl Beat {
    /// Lock this server's file under `<data_dir>/mcp`.
    ///
    /// # Errors
    /// The dir or file cannot be made, or the lock is held already.
    #[allow(clippy::unnecessary_wraps, reason = "RED stub")]
    pub(super) fn start(
        _data_dir: &Path,
        _project: &str,
        _pid: u32,
    ) -> io::Result<Self> {
        Ok(Self { recent: Vec::new() })
    }

    /// Note 1 reply that `took` and write the beat at `now` (ms).
    ///
    /// # Errors
    /// The beat file cannot be written.
    #[allow(clippy::unnecessary_wraps, reason = "RED stub")]
    pub(super) fn record(
        &mut self,
        took: Duration,
        _now: u64,
    ) -> io::Result<()> {
        self.recent.push(took);
        Ok(())
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
/// as a badge must not fail.
#[must_use]
pub fn pulse(_data_dir: &Path, _project: &str) -> Pulse {
    Pulse::Dead
}
