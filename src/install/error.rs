//! Why an install call failed; each message reads `chert: <what>
//! (<fix>)`, as `store::Error` does (src C).

#[cfg(test)]
mod tests;

use std::fmt;
use std::path::PathBuf;

use crate::store;

/// An install, uninstall, status or doctor failure (src/install I).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The IDE config or its backup could not be read or written.
    Io { path: PathBuf, reason: String },
    /// The IDE config is not JSON: left as it is, never overwritten.
    Json { path: PathBuf, reason: String },
    /// A key chert edits holds the wrong JSON type, e.g. `hooks` that is
    /// no object: left as it is.
    Shape { path: PathBuf, key: String },
    /// The store `doctor` checks failed.
    Store(store::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(e) => e.fmt(f),
            other => write!(f, "chert: {} ({})", other.what(), other.fix()),
        }
    }
}

impl std::error::Error for Error {}

impl From<store::Error> for Error {
    fn from(e: store::Error) -> Self {
        Self::Store(e)
    }
}

impl Error {
    fn what(&self) -> String {
        match self {
            Self::Io { path, reason } | Self::Json { path, reason } => {
                format!("{}: {reason}", path.display())
            }
            Self::Shape { path, key } => {
                format!("{}: `{key}` has the wrong JSON type", path.display())
            }
            Self::Store(e) => e.to_string(),
        }
    }

    fn fix(&self) -> &'static str {
        match self {
            Self::Io { .. } => "check the path and its permissions",
            Self::Json { .. } | Self::Shape { .. } => {
                "fix the file by hand, then run it again"
            }
            Self::Store(_) => "run chert doctor",
        }
    }
}
