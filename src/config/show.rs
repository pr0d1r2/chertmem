//! `chert config show` and `get`: every key's effective value and the
//! file it came from (src/config T6, V1).

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use super::Error;

/// Where an effective value came from (src/config V1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// No file sets the key: the root C default.
    Default,
    /// `~/.config/chert/chert.toml`.
    User(PathBuf),
    /// `<project_root>/chert.toml`.
    Project(PathBuf),
}

/// 1 key's effective value, as TOML text, and its [`Origin`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: &'static str,
    /// The value as TOML, e.g. `"28d"`; `None` only for a default
    /// `storage.data_dir` without a home.
    pub value: Option<String>,
    pub origin: Origin,
}

/// The root C defaults as TOML; `storage.data_dir` hangs off the home.
const DEFAULTS: &[(&str, &str)] = &[
    ("partition.by", "[\"project\", \"time\"]"),
    ("partition.time_bucket", "\"7d\""),
    ("partition.shards", "1"),
    ("cache.window", "\"28d\""),
    ("cache.max_bytes", "\"256MiB\""),
    ("cache.on_overflow", "\"warn\""),
    ("compression.grammar", "\"full\""),
    ("compression.zstd.level", "3"),
    ("compression.zstd.dictionary", "true"),
    ("search.alpha", "0.5"),
    ("search.default_limit", "10"),
    ("embedding.provider", "\"off\""),
    ("privacy.exclude", "[]"),
    ("mining.enabled", "true"),
    ("mining.min_occurrences", "3"),
    ("inject.max_tokens", "300"),
    ("inject.on_prompt", "false"),
];

/// Every leaf key with its effective value; a stub, values all unset.
///
/// # Errors
/// None yet.
pub fn show(
    home: Option<&Path>,
    project_root: &Path,
) -> Result<Vec<Entry>, Error> {
    let _ = (home, project_root);
    let stub = |&(key, _): &(&'static str, &str)| Entry {
        key,
        value: None,
        origin: Origin::Default,
    };
    Ok(DEFAULTS.iter().map(stub).collect())
}

/// 1 key of [`show`]; a stub, never found.
///
/// # Errors
/// None yet.
pub fn get(
    home: Option<&Path>,
    project_root: &Path,
    key: &str,
) -> Result<Option<Entry>, Error> {
    let _ = (home, project_root, key);
    Ok(None)
}
