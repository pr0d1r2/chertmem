//! `status` and `doctor` (src/install T2, V3): chert's wiring read back
//! from the IDE config, then the store checked for catalog drift
//! (`src/store:V6`) and stale spool files.

#[cfg(test)]
mod tests;

use std::fmt;
use std::path::{Path, PathBuf};

use super::{Error, Target, claude_code};
use crate::config::Config;
use crate::hook::Ide;

/// 1 entry chert writes, e.g. `hook SessionStart`, and how it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wire {
    pub name: String,
    pub state: WireState,
}

/// How a [`Wire`] stands in the config file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireState {
    /// There, running this `chert`.
    Ok,
    /// Not there: `chert install` adds it.
    Missing,
    /// chert's entry, but not the one this `chert` writes (another
    /// binary path): the entry as found.
    Other(String),
}

/// What `chert status` prints: each entry of the config file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub settings: PathBuf,
    pub wires: Vec<Wire>,
}

impl Status {
    /// Whether every entry is there and runs this `chert`.
    #[must_use]
    pub fn wired(&self) -> bool {
        self.wires.iter().all(|w| w.state == WireState::Ok)
    }
}

/// How the catalog differed from the partition files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drift {
    /// A catalog row whose file is gone: now marked dropped.
    Gone,
    /// A file the catalog did not list: listed now.
    Unlisted,
    /// A catalog row unlike its file's own: rewritten.
    Differs,
}

/// 1 thing `doctor` found wrong (V3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// An entry missing from the config, or running another `chert`.
    Wiring(Wire),
    /// A partition the catalog had wrong, by its catalog path; the
    /// catalog is rebuilt from the files (`src/store:V6`).
    Drift { path: String, drift: Drift },
    /// Files left in a spool dir after a full replay: never replayed.
    Spool { dir: PathBuf, files: usize },
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("")
    }
}

/// What `chert doctor` prints, and its exit code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub findings: Vec<Finding>,
}

impl Report {
    /// 0 when nothing was found, else 1 (root I, V3).
    #[must_use]
    pub fn exit(&self) -> u8 {
        u8::from(!self.findings.is_empty())
    }
}

/// Each of chert's entries for `ide` in `target`; a stub until
/// src/install T2 lands.
///
/// # Errors
/// None yet.
pub fn status(target: &Target<'_>, ide: Ide) -> Result<Status, Error> {
    let wires = match ide {
        Ide::ClaudeCode => claude_code::wiring(&serde_json::Value::Null, ""),
    };
    Ok(Status {
        settings: target.settings.to_path_buf(),
        wires,
    })
}

/// Every broken wiring, catalog drift and stale spool file (V3); a stub
/// until src/install T2 lands.
///
/// # Errors
/// None yet.
pub fn doctor(
    target: &Target<'_>,
    ide: Ide,
    config: &Config,
    root: &Path,
) -> Result<Report, Error> {
    let _ = (target, ide, config, root);
    Ok(Report {
        findings: Vec::new(),
    })
}
