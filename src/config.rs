//! `chert.toml` from user & project scope, merged key by key (project
//! wins), defaults applied and every value validated (src/config G).
//!
//! RED stub (src/config T1): the types, with answers still wrong.

#[cfg(test)]
mod tests;

use std::fmt;
use std::path::{Path, PathBuf};

/// Why `chert.toml` could not be loaded; each names the file (src/config
/// V3, root C `chert: <what> (<fix>)`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The file exists but could not be read.
    Read { file: PathBuf, reason: String },
    /// The file is not valid TOML.
    Parse { file: PathBuf, reason: String },
    /// A key outside src/config I.
    UnknownKey { file: PathBuf, key: String },
    /// A known key with a value outside its accepted forms.
    Invalid {
        file: PathBuf,
        key: String,
        value: String,
        accepted: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.file().display())
    }
}

impl Error {
    /// The file the error is about.
    #[must_use]
    pub fn file(&self) -> &Path {
        match self {
            Self::Read { file, .. }
            | Self::Parse { file, .. }
            | Self::UnknownKey { file, .. }
            | Self::Invalid { file, .. } => file,
        }
    }
}

impl std::error::Error for Error {}

/// The unit of a [`Span`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Hours,
    Days,
    Months,
}

/// A duration as written: `<n>d`, `<n>h` or `<n>mo` (src/config C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub amount: u32,
    pub unit: Unit,
}

impl Span {
    /// Parse `<n>d`, `<n>h` or `<n>mo`, n >= 1.
    #[must_use]
    pub fn parse(_text: &str) -> Option<Self> {
        None
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.amount)
    }
}

/// Parse a size `<n>KiB`, `<n>MiB` or `<n>GiB`, n >= 1, into bytes.
#[must_use]
pub fn parse_size(_text: &str) -> Option<u64> {
    None
}

/// A `partition.by` dimension (src/config C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dimension {
    Project,
    Time,
    Ide,
}

/// `cache.on_overflow` (src/cache C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    Warn,
    Shrink,
    Fail,
}

/// `compression.grammar`, the caveman intensity (src/grammar G).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grammar {
    Off,
    Lite,
    Full,
    Ultra,
}

/// `embedding.provider` (root C, D18).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Off,
    Ollama,
    Openai,
}

/// `storage.*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Storage {
    /// Default `~/.local/share/chert` (root I); `None` without a home.
    pub data_dir: Option<PathBuf>,
}

/// `partition.*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    pub by: Vec<Dimension>,
    pub time_bucket: Span,
    pub shards: u32,
}

/// `cache.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cache {
    pub window: Span,
    pub max_bytes: u64,
    pub on_overflow: Overflow,
}

/// `compression.*`; `None` = unset, no default in root C, so the owning
/// module decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Compression {
    pub grammar: Option<Grammar>,
    pub zstd: Option<bool>,
}

/// `search.*`; `None` = unset, the owning module decides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Search {
    pub alpha: Option<f64>,
    pub default_limit: Option<u32>,
}

/// `embedding.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Embedding {
    pub provider: Provider,
}

/// `privacy.*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Privacy {
    pub exclude: Vec<String>,
}

/// `mining.*`; `None` = unset, the owning module decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mining {
    pub enabled: Option<bool>,
    pub min_occurrences: Option<u32>,
}

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
    _home: Option<&Path>,
    project_root: &Path,
) -> Result<Loaded, Error> {
    let file = project_root.join("chert.toml");
    let reason = "not loaded yet".to_owned();
    Err(Error::Read { file, reason })
}
