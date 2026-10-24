//! `storage.*` (src/config I).

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use super::Error;
use super::parse::PATH;
use super::source::Source;

/// `storage.*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Storage {
    /// Default `~/.local/share/chert` (root I); `None` without a home.
    pub data_dir: Option<PathBuf>,
}

impl Storage {
    pub(super) fn build(
        src: &Source,
        home: Option<&Path>,
    ) -> Result<Self, Error> {
        let set = src
            .get("storage.data_dir", PATH, |v| v.as_str().map(PathBuf::from))?;
        let default = || home.map(|h| h.join(".local/share/chert"));
        Ok(Self {
            data_dir: set.or_else(default),
        })
    }
}
