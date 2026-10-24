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
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(e) => e.fmt(f),
            Self::Cavemem { reason } => write!(
                f,
                "chert: cavemem db unreadable: {reason} (give a cavemem \
                 0.2.1 db)"
            ),
            Self::Io { reason } => write!(
                f,
                "chert: export failed: {reason} (check the path and its \
                 permissions)"
            ),
        }
    }
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

impl From<store::Error> for Error {
    fn from(e: store::Error) -> Self {
        Self::Store(e)
    }
}
