//! `chert.toml` from user & project scope, merged key by key (project
//! wins), defaults applied and every value validated (src/config G).

#[cfg(test)]
mod tests;

mod cache;
mod compression;
mod edit;
mod error;
mod inject;
mod parse;
mod partition;
mod sections;
mod show;
mod source;
mod storage;

use std::path::{Path, PathBuf};

pub use cache::{Cache, Overflow};
pub use compression::{Compression, Grammar, Zstd};
pub use edit::set;
pub use error::Error;
pub use inject::{Inject, ON_PROMPT_DISCLAIMER};
pub use parse::{Span, Unit, parse_size};
pub use partition::{Dimension, Partition};
pub use sections::{Embedding, Mining, Privacy, Provider, Search};
pub use show::{Entry, Origin, get, show};
pub use source::{Paths, paths};
pub use storage::Storage;

use cache::window_warning;
use source::{Source, files};

/// The merged, validated `chert.toml` (src/config G).
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub storage: Storage,
    pub partition: Partition,
    pub cache: Cache,
    pub compression: Compression,
    pub search: Search,
    pub embedding: Embedding,
    pub privacy: Privacy,
    pub mining: Mining,
    pub inject: Inject,
}

/// A loaded [`Config`] and the warnings found on the way (src/config V2).
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub config: Config,
    pub warnings: Vec<String>,
}

/// Load `~/.config/chert/chert.toml` and `<project_root>/chert.toml`
/// (src/config I), `$HOME` naming the home directory.
///
/// # Errors
/// An unreadable or invalid file, an unknown key or a bad value.
pub fn load(project_root: &Path) -> Result<Loaded, Error> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    load_from(home.as_deref(), project_root)
}

/// [`load`] with the home directory given; `None` skips the user file.
///
/// # Errors
/// An unreadable or invalid file, an unknown key or a bad value.
pub fn load_from(
    home: Option<&Path>,
    project_root: &Path,
) -> Result<Loaded, Error> {
    let source = Source::gather(&files(home, project_root))?;
    let config = Config::build(&source, home)?;
    let (window, bucket) = (config.cache.window, config.partition.time_bucket);
    let warnings = window_warning(window, bucket).into_iter().collect();
    Ok(Loaded { config, warnings })
}

impl Config {
    fn build(src: &Source, home: Option<&Path>) -> Result<Self, Error> {
        Ok(Self {
            storage: Storage::build(src, home)?,
            partition: Partition::build(src)?,
            cache: Cache::build(src)?,
            compression: Compression::build(src)?,
            search: Search::build(src)?,
            embedding: Embedding::build(src)?,
            privacy: Privacy::build(src)?,
            mining: Mining::build(src)?,
            inject: Inject::build(src)?,
        })
    }
}
