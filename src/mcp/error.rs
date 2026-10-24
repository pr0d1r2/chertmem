//! Why `chert mcp` stopped; each message reads `chert: <what> (<fix>)`
//! (src C). A failed tool call is not one: it answers `isError`.

#[cfg(test)]
mod tests;

use std::fmt;

use crate::{search, store};

/// A server failure; local until the `src` crate error enum lands, as
/// `store::Error` is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The store could not be opened for the cache.
    Store(store::Error),
    /// The search engine could not be opened.
    Search(search::Error),
    /// Reading stdin or writing stdout failed.
    Io { reason: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(e) => e.fmt(f),
            Self::Search(e) => e.fmt(f),
            Self::Io { reason } => write!(
                f,
                "chert: mcp stdio failed: {reason} (restart chert mcp)"
            ),
        }
    }
}

impl std::error::Error for Error {}

impl From<store::Error> for Error {
    fn from(e: store::Error) -> Self {
        Self::Store(e)
    }
}

impl From<search::Error> for Error {
    fn from(e: search::Error) -> Self {
        Self::Search(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io {
            reason: e.to_string(),
        }
    }
}
