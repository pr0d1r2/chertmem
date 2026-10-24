//! Why a cache call failed; each message reads `chert: <what> (<fix>)`
//! (src C).

#[cfg(test)]
mod tests;

use std::fmt;

use crate::store;

/// A cache failure; local until the `src` crate error enum lands, as
/// `store::Error` is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The store under the cache failed.
    Store(store::Error),
    /// A thread panicked holding the cache lock.
    Poisoned,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(e) => e.fmt(f),
            Self::Poisoned => f.write_str(
                "chert: cache lock poisoned by a panicked thread \
                 (restart chert mcp)",
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
