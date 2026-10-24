//! The two `chert.toml` files read, flattened to `section.key` and
//! merged key by key, the project file winning (src/config V1).

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use toml::{Table, Value};

use super::Error;
use super::parse::name;

/// Every key `chert.toml` accepts (src/config I); any other is an error.
const KEYS: &[&str] = &[
    "storage.data_dir",
    "partition.by",
    "partition.time_bucket",
    "partition.shards",
    "cache.window",
    "cache.max_bytes",
    "cache.on_overflow",
    "compression.grammar",
    "compression.zstd",
    "compression.zstd.level",
    "compression.zstd.dictionary",
    "search.alpha",
    "search.default_limit",
    "embedding.provider",
    "privacy.exclude",
    "mining.enabled",
    "mining.min_occurrences",
    "inject.max_tokens",
    "inject.on_prompt",
];

/// Keys whose value is a table of keys of their own, each merged and
/// defaulted on its own (root C `compression.zstd`).
const TABLES: &[&str] = &["compression.zstd"];

/// The user file (when there is a home), then the project file: the
/// later wins (V1).
pub(super) fn files(home: Option<&Path>, project_root: &Path) -> Vec<PathBuf> {
    let user = home.map(|h| h.join(".config/chert/chert.toml"));
    let project = project_root.join("chert.toml");
    user.into_iter().chain(std::iter::once(project)).collect()
}

/// The file's table; `None` when it does not exist.
fn read(file: &Path) -> Result<Option<Table>, Error> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            let reason = e.to_string();
            return Err(Error::Read {
                file: file.to_path_buf(),
                reason,
            });
        }
    };
    text.parse::<Table>().map(Some).map_err(|e| Error::Parse {
        file: file.to_path_buf(),
        reason: e.to_string().trim().to_owned(),
    })
}

/// A value and the file it came from.
#[derive(Debug)]
struct Entry {
    value: Value,
    file: PathBuf,
}

/// The merged `section.key` values; a later file wins per key (V1).
#[derive(Debug, Default)]
pub(super) struct Source {
    entries: BTreeMap<String, Entry>,
}

impl Source {
    pub(super) fn gather(files: &[PathBuf]) -> Result<Self, Error> {
        let mut source = Self::default();
        for file in files {
            if let Some(table) = read(file)? {
                source.merge(file, table)?;
            }
        }
        Ok(source)
    }

    fn merge(&mut self, file: &Path, table: Table) -> Result<(), Error> {
        for (key, value) in flatten(table) {
            self.insert(file, key, value)?;
        }
        Ok(())
    }

    fn insert(
        &mut self,
        file: &Path,
        key: String,
        value: Value,
    ) -> Result<(), Error> {
        if !KEYS.contains(&key.as_str()) {
            return Err(Error::UnknownKey {
                file: file.to_path_buf(),
                key,
            });
        }
        let file = file.to_path_buf();
        self.entries.insert(key, Entry { value, file });
        Ok(())
    }

    /// The key's parsed value; `None` when no file sets it (V1), an
    /// error when `parse` rejects it (V3).
    pub(super) fn get<T>(
        &self,
        key: &str,
        accepted: &str,
        parse: impl Fn(&Value) -> Option<T>,
    ) -> Result<Option<T>, Error> {
        let Some(entry) = self.entries.get(key) else {
            return Ok(None);
        };
        parse(&entry.value).map(Some).ok_or_else(|| Error::Invalid {
            file: entry.file.clone(),
            key: key.to_owned(),
            value: entry.value.to_string(),
            accepted: accepted.to_owned(),
        })
    }

    /// A key whose value is one of `names`.
    pub(super) fn pick<T: Copy>(
        &self,
        key: &str,
        names: &[(&str, T)],
    ) -> Result<Option<T>, Error> {
        let forms: Vec<String> =
            names.iter().map(|(n, _)| format!("\"{n}\"")).collect();
        self.get(key, &forms.join(", "), |v| name(v, names))
    }
}

/// `section.key` pairs; a top-level value keeps its bare name.
fn flatten(table: Table) -> Vec<(String, Value)> {
    let pairs = table.into_iter().flat_map(|(section, value)| match value {
        Value::Table(inner) => inner
            .into_iter()
            .flat_map(|(key, v)| nest(format!("{section}.{key}"), v))
            .collect(),
        other => vec![(section, other)],
    });
    pairs.collect()
}

/// A table under a key in [`TABLES`] spreads into `key.sub` pairs; any
/// other value stays whole.
fn nest(key: String, value: Value) -> Vec<(String, Value)> {
    match value {
        Value::Table(inner) if TABLES.contains(&key.as_str()) => inner
            .into_iter()
            .map(|(sub, v)| (format!("{key}.{sub}"), v))
            .collect(),
        other => vec![(key, other)],
    }
}
