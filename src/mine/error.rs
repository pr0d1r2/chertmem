//! Why mining failed; each message reads `chert: <what> (<fix>)` (src
//! C).

#[cfg(test)]
mod tests;

use std::fmt;
use std::path::{Path, PathBuf};

use crate::store;

/// A mining failure (src/mine I).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The store refused a read.
    Store(store::Error),
    /// The finding store `mine/findings.db` under the data dir could not
    /// be used.
    Ledger { path: PathBuf, reason: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(e) => e.fmt(f),
            Self::Ledger { path, reason } => write!(
                f,
                "chert: finding store {} unusable: {reason} (remove it; \
                 `chert mine` mines every partition again)",
                path.display()
            ),
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    /// A failure of the finding store at `path`.
    pub(super) fn ledger(path: &Path, reason: impl fmt::Display) -> Self {
        Self::Ledger {
            path: path.to_path_buf(),
            reason: reason.to_string(),
        }
    }
}

impl From<store::Error> for Error {
    fn from(e: store::Error) -> Self {
        Self::Store(e)
    }
}
