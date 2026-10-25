//! `status` and `doctor` (src/install T2, V3): chert's wiring read back
//! from the IDE config, then the store checked for catalog drift
//! (`src/store:V6`) and stale spool files.

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::{Error, Target, claude_code, settings};
use crate::config::Config;
use crate::hook::Ide;
use crate::store::{self, Filter, Mode, Partition, Store};

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

/// What `chert status` prints: each entry of the 2 config files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub settings: PathBuf,
    pub mcp: PathBuf,
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
        match self {
            Self::Wiring(w) => wire_line(f, w),
            Self::Drift { path, drift } => drift_line(f, path, *drift),
            Self::Spool { dir, files } => write!(
                f,
                "stale spool: {files} file(s) in {} never replayed",
                dir.display()
            ),
        }
    }
}

fn wire_line(f: &mut fmt::Formatter<'_>, w: &Wire) -> fmt::Result {
    let name = &w.name;
    match &w.state {
        WireState::Ok => write!(f, "wiring {name}: ok"),
        WireState::Missing => {
            write!(f, "wiring {name}: missing (run chert install)")
        }
        WireState::Other(found) => write!(
            f,
            "wiring {name}: runs another chert: {found} (run chert install)"
        ),
    }
}

fn drift_line(f: &mut fmt::Formatter<'_>, path: &str, d: Drift) -> fmt::Result {
    let what = match d {
        Drift::Gone => "catalog row, file gone; marked dropped",
        Drift::Unlisted => "file missing from the catalog; listed again",
        Drift::Differs => "catalog row unlike the file; rebuilt",
    };
    write!(f, "catalog drift: {path}: {what}")
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

/// Each of chert's entries for `ide` in `target`: hooks read from its
/// settings file, the server from its MCP file (V4).
///
/// # Errors
/// A config file is unreadable or not JSON.
pub fn status(target: &Target<'_>, ide: Ide) -> Result<Status, Error> {
    let hooks = settings::load(target.settings)?.value;
    let servers = settings::load(target.mcp)?.value;
    let wires = match ide {
        Ide::ClaudeCode => claude_code::wiring(&hooks, &servers, target.chert),
    };
    Ok(Status {
        settings: target.settings.to_path_buf(),
        mcp: target.mcp.to_path_buf(),
        wires,
    })
}

/// Every broken wiring of `ide` in `target`, then the store of
/// `config` for project `root`: catalog drift, which it repairs by
/// rebuilding the catalog from the files, and stale spool files (V3).
/// A data dir that is not there yet has nothing to check.
///
/// # Errors
/// The config is unreadable or not JSON; the store fails.
pub fn doctor(
    target: &Target<'_>,
    ide: Ide,
    config: &Config,
    root: &Path,
) -> Result<Report, Error> {
    let wires = status(target, ide)?.wires.into_iter();
    let broken = wires.filter(|w| w.state != WireState::Ok);
    let mut findings: Vec<Finding> = broken.map(Finding::Wiring).collect();
    findings.extend(in_store(config, root)?);
    Ok(Report { findings })
}

/// The store's findings: drift, then stale spool.
fn in_store(config: &Config, root: &Path) -> Result<Vec<Finding>, Error> {
    let data = config.storage.data_dir.as_deref();
    let data = data.ok_or(store::Error::NoDataDir)?;
    if !data.exists() {
        return Ok(Vec::new());
    }
    let mut store = Store::open(config, root, Mode::Read)?;
    let mut found = drift(&mut store)?;
    found.extend(stale(data)?);
    Ok(found)
}

/// Every live catalog row by path.
fn rows(store: &Store) -> Result<BTreeMap<String, Partition>, Error> {
    let all = store.partitions(&Filter::default())?;
    Ok(all.into_iter().map(|p| (p.path.clone(), p)).collect())
}

/// The catalog before and after a rebuild from the files, compared
/// row by row.
fn drift(store: &mut Store) -> Result<Vec<Finding>, Error> {
    let before = rows(store)?;
    store.rebuild()?;
    let after = rows(store)?;
    let paths: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    let compared = paths.into_iter().map(|p| (p, before.get(p), after.get(p)));
    Ok(compared.filter_map(|(p, b, a)| compare(p, b, a)).collect())
}

fn compare(
    path: &str,
    before: Option<&Partition>,
    after: Option<&Partition>,
) -> Option<Finding> {
    let drift = match (before, after) {
        (Some(_), None) => Drift::Gone,
        (None, Some(_)) => Drift::Unlisted,
        (Some(b), Some(a)) if b != a => Drift::Differs,
        _ => return None,
    };
    let path = path.to_owned();
    Some(Finding::Drift { path, drift })
}

/// The spool dirs no replay empties, and `spool/` itself, whose files
/// outlived the reading open's replay (`src/store:V10`).
const SPOOL: [&str; 3] = ["spool", "spool/sealed", "spool/unreadable"];

fn stale(data: &Path) -> Result<Vec<Finding>, Error> {
    let mut found = Vec::new();
    for dir in SPOOL.map(|d| data.join(d)) {
        let files = count(&dir)?;
        if files > 0 {
            found.push(Finding::Spool { dir, files });
        }
    }
    Ok(found)
}

/// The files in `dir`, temp files (`.<name>`) aside; 0 when no dir.
fn count(dir: &Path) -> Result<usize, Error> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(Error::io(dir, &e)),
    };
    let named = |e: &std::fs::DirEntry| {
        let dot = e.file_name().to_string_lossy().starts_with('.');
        !dot && e.path().is_file()
    };
    Ok(entries.filter_map(Result::ok).filter(named).count())
}
