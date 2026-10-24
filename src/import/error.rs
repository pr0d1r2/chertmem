//! Why an export or an import failed; each message reads `chert: <what>
//! (<fix>)` (src C).

#[cfg(test)]
mod tests;

use std::fmt;

use crate::store;

/// An export or import failure (src/import I).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The chert store refused a read or a write.
    Store(store::Error),
    /// The cavemem file could not be read as cavemem 0.2.1's schema.
    Cavemem { reason: String },
    /// The export could not be written.
    Io { reason: String },
    /// A JSONL line that is not 1 line of a `chert export`.
    Jsonl { reason: String },
    /// Line `line` (from 1) of a JSONL import failed with `cause`.
    Line { line: usize, cause: Box<Error> },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ((what, fix), reason) = match self {
            Self::Store(e) => return e.fmt(f),
            Self::Line { line, cause } => return at_line(f, *line, cause),
            Self::Cavemem { reason } => (CAVEMEM, reason),
            Self::Io { reason } => (IO, reason),
            Self::Jsonl { reason } => (JSONL, reason),
        };
        write!(f, "chert: {what}: {reason} ({fix})")
    }
}

/// What failed and its fix, per message (src C).
const CAVEMEM: (&str, &str) =
    ("cavemem db unreadable", "give a cavemem 0.2.1 db");
const IO: (&str, &str) =
    ("export failed", "check the path and its permissions");
const JSONL: (&str, &str) =
    ("not an export line", "give a file `chert export` wrote");

/// `cause`'s message with `import line <line>: ` after its `chert: `.
fn at_line(
    f: &mut fmt::Formatter<'_>,
    line: usize,
    cause: &Error,
) -> fmt::Result {
    let text = cause.to_string();
    let rest = text.strip_prefix("chert: ").unwrap_or(&text);
    write!(f, "chert: import line {line}: {rest}")
}

impl std::error::Error for Error {}

impl Error {
    /// A write failure on the export.
    pub(super) fn io(e: &std::io::Error) -> Self {
        Self::Io {
            reason: e.to_string(),
        }
    }
}

/// Only the cavemem file is read through a raw connection; the store
/// reports its own `SQLite` failures as [`store::Error`].
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::Cavemem {
            reason: e.to_string(),
        }
    }
}

impl From<store::Error> for Error {
    fn from(e: store::Error) -> Self {
        Self::Store(e)
    }
}
