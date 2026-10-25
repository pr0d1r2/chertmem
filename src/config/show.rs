//! `chert config show` and `get`: every key's effective value and the
//! file it came from (src/config T6, V1).

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use toml::Value;

use super::source::{KEYS, Source, TABLES, files, paths};
use super::{Config, Error};

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
    ("statusline.ms_green", "10"),
    ("statusline.ms_amber", "50"),
    ("statusline.size_green", "\"1GiB\""),
    ("statusline.size_amber", "\"5GiB\""),
    ("statusline.fit_amber", "80"),
];

/// Every leaf key, in src/config I order, with its effective value;
/// the files are validated as [`super::load_from`] does.
///
/// # Errors
/// An unreadable or invalid file, an unknown key or a bad value.
pub fn show(
    home: Option<&Path>,
    project_root: &Path,
) -> Result<Vec<Entry>, Error> {
    let source = Source::gather(&files(home, project_root))?;
    Config::build(&source, home)?;
    let project = paths(home, project_root).project;
    let leaves = KEYS.iter().copied().filter(|k| !TABLES.contains(k));
    let entry = |key| effective(&source, key, home, &project);
    Ok(leaves.map(entry).collect())
}

/// 1 key of [`show`]; `None` when `key` is not a leaf key.
///
/// # Errors
/// As [`show`].
pub fn get(
    home: Option<&Path>,
    project_root: &Path,
    key: &str,
) -> Result<Option<Entry>, Error> {
    let entries = show(home, project_root)?;
    Ok(entries.into_iter().find(|e| e.key == key))
}

fn effective(
    source: &Source,
    key: &'static str,
    home: Option<&Path>,
    project: &Path,
) -> Entry {
    let Some((value, file)) = source.origin(key) else {
        let value = default_text(key, home);
        let origin = Origin::Default;
        return Entry { key, value, origin };
    };
    let origin = if file == project {
        Origin::Project(file.to_path_buf())
    } else {
        Origin::User(file.to_path_buf())
    };
    let value = Some(value.to_string());
    Entry { key, value, origin }
}

fn default_text(key: &str, home: Option<&Path>) -> Option<String> {
    if key == "storage.data_dir" {
        let dir = home?.join(".local/share/chert");
        let text = dir.to_string_lossy().into_owned();
        return Some(Value::String(text).to_string());
    }
    let found = DEFAULTS.iter().find(|(k, _)| *k == key);
    found.map(|(_, text)| (*text).to_owned())
}
