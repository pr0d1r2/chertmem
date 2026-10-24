//! Why a store call failed; each message reads `chert: <what> (<fix>)`
//! (src C).

#[cfg(test)]
mod tests;

use std::fmt;
use std::path::{Path, PathBuf};

use rusqlite::ErrorCode;

/// A store failure (src/store I); local until the `src` crate error enum
/// lands, as `config::Error` is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// `storage.data_dir` is unset and there is no home to default it.
    NoDataDir,
    /// A file or directory under the data dir could not be used.
    Io { path: PathBuf, reason: String },
    /// `SQLite` refused a statement.
    Sqlite { reason: String },
    /// A lock was still held when the busy timeout ran out.
    Busy { reason: String },
    /// A read on a store opened write-only (src/store V10).
    WriteOnly,
    /// A write into a warm or cold partition (src/store V7).
    ReadOnly { path: String },
    /// Text that is not `chert:<ref>`.
    BadRef { text: String },
    /// A ref issued in more than one project (root V15).
    Ambiguous {
        reference: String,
        projects: Vec<String>,
    },
    /// A timestamp a ULID cannot hold (48 bits of milliseconds).
    BadTime { ts: u64 },
    /// A restored row's ref is already issued to another id in its
    /// project (root V15).
    RefTaken { reference: String, id: String },
    /// No new id at `ts`: its ids ran out in this process, or the id
    /// lock is poisoned (src/store T15).
    NoId { ts: u64, reason: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "chert: {} ({})", self.what(), self.fix())
    }
}

impl std::error::Error for Error {}

impl Error {
    /// An I/O failure on `path`.
    pub(super) fn io(path: &Path, e: &std::io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            reason: e.to_string(),
        }
    }

    fn what(&self) -> String {
        let fixed = match self {
            Self::NoDataDir => "no data dir",
            Self::WriteOnly => "store opened write-only",
            other => return other.detail(),
        };
        fixed.to_owned()
    }

    fn detail(&self) -> String {
        match self {
            Self::Io { path, reason } => {
                format!("{}: {reason}", path.display())
            }
            Self::Sqlite { reason } => format!("store failed: {reason}"),
            Self::Busy { reason } => format!("store is busy: {reason}"),
            Self::ReadOnly { path } => format!("partition {path} is sealed"),
            Self::BadRef { text } => format!("`{text}` is not a ref"),
            Self::Ambiguous {
                reference: r,
                projects: p,
            } => format!("{r} is in {}", p.join(", ")),
            other => other.clash(),
        }
    }

    /// The message of a bad time or a ref clash.
    fn clash(&self) -> String {
        match self {
            Self::BadTime { ts } => format!("timestamp {ts} is out of range"),
            Self::RefTaken { reference: r, id } => {
                format!("chert:{r} is issued to another row than {id}")
            }
            Self::NoId { ts, reason } => format!("no new id at {ts}: {reason}"),
            _ => String::new(),
        }
    }

    fn fix(&self) -> &'static str {
        match self {
            Self::NoDataDir => "set storage.data_dir or HOME",
            Self::Io { .. } => "check the path and its permissions",
            Self::Sqlite { .. } => "run chert doctor",
            Self::Busy { .. } => "retry",
            Self::WriteOnly => "open the store for reading",
            Self::ReadOnly { .. } => "write into a hot partition",
            Self::BadRef { .. } => "give chert:<ref> as issued",
            Self::Ambiguous { .. } => "give the project",
            Self::BadTime { .. } => "give milliseconds below 2^48",
            Self::RefTaken { .. } => "restore into a store without that ref",
            Self::NoId { .. } => "retry in a new process",
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        let reason = e.to_string();
        match e.sqlite_error_code() {
            Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked) => {
                Self::Busy { reason }
            }
            _ => Self::Sqlite { reason },
        }
    }
}
