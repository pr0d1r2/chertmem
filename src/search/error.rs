//! Why a search failed; each message reads `chert: <what> (<fix>)`
//! (src C).

#[cfg(test)]
mod tests;

use std::fmt;
use std::path::Path;

use crate::{cache, store};

/// A search failure; local until the `src` crate error enum lands, as
/// `store::Error` is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The store under the search failed.
    Store(store::Error),
    /// The cache answering the search failed.
    Cache(cache::Error),
    /// The search's own term file failed (src/search R1).
    Terms { reason: String },
    /// A worker thread querying partitions panicked.
    Thread,
    /// The embedding provider failed or answered nonsense.
    Embed { reason: String },
    /// A provider `chert.toml` allows but v1 does not ship.
    Unsupported { provider: &'static str },
    /// The `around` ref of a timeline is gone (root V5).
    Dropped { reference: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(e) => e.fmt(f),
            Self::Cache(e) => e.fmt(f),
            other => write!(f, "chert: {} ({})", other.what(), other.fix()),
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    /// An I/O failure on the term file at `path`.
    pub(super) fn io(path: &Path, e: &std::io::Error) -> Self {
        Self::Terms {
            reason: format!("{}: {e}", path.display()),
        }
    }

    fn what(&self) -> String {
        match self {
            Self::Terms { reason } => format!("term stats failed: {reason}"),
            Self::Thread => "a search thread panicked".to_owned(),
            Self::Embed { reason } => format!("embedding failed: {reason}"),
            Self::Unsupported { provider } => {
                format!("embedding.provider {provider} is not in v1")
            }
            Self::Dropped { reference } => format!("{reference} is dropped"),
            Self::Store(_) | Self::Cache(_) => String::new(),
        }
    }

    fn fix(&self) -> &'static str {
        match self {
            Self::Terms { .. } => "remove the search dir; it is rebuilt",
            Self::Thread => "retry",
            Self::Embed { .. } => "check OLLAMA_HOST or pass --no-semantic",
            Self::Unsupported { .. } => "set ollama or off",
            Self::Dropped { .. } => "give a ref still stored",
            Self::Store(_) | Self::Cache(_) => "",
        }
    }
}

impl From<store::Error> for Error {
    fn from(e: store::Error) -> Self {
        Self::Store(e)
    }
}

impl From<cache::Error> for Error {
    fn from(e: cache::Error) -> Self {
        Self::Cache(e)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::Terms {
            reason: e.to_string(),
        }
    }
}
