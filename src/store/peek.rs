//! A read-only look at the catalog for `chert statusline` (src/cli
//! T3): no dir made, no spool replayed, no partition opened, so a badge
//! drawn every turn costs 1 catalog query. Rows still in the spool are
//! not counted until a reading open replays them (V10). RED stub: never
//! a catalog.

#[cfg(test)]
mod tests;

use std::path::Path;

use super::{Error, Store};
use crate::config::Config;

impl Store {
    /// The catalog of `cfg`'s data dir for the project at
    /// `project_root`, opened read-only; `None` without a data dir or a
    /// catalog. Only catalog reads work on it: an append fails.
    ///
    /// # Errors
    /// An unreadable catalog.
    #[allow(clippy::unnecessary_wraps, reason = "RED stub")]
    pub fn peek(
        _cfg: &Config,
        _project_root: &Path,
    ) -> Result<Option<Self>, Error> {
        Ok(None)
    }
}
