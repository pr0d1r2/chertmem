//! `chert.toml` from user & project scope, merged key by key (project
//! wins), defaults applied and every value validated (src/config G).

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::fmt;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use toml::{Table, Value};

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
    "search.alpha",
    "search.default_limit",
    "embedding.provider",
    "privacy.exclude",
    "mining.enabled",
    "mining.min_occurrences",
];

const SPAN_FORMS: &str = "<n>d, <n>h or <n>mo with n >= 1";
const SIZE_FORMS: &str = "<n>KiB, <n>MiB or <n>GiB with n >= 1";
const BY_FORMS: &str =
    "an array of distinct \"project\", \"time\", \"ide\" holding \"time\"";
const POSITIVE: &str = "an integer >= 1";
const BOOL: &str = "true or false";
const ALPHA: &str = "a number from 0 to 1";
const PATH: &str = "a path string";
const PATHS: &str = "an array of path strings";

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
        write!(f, "chert: {} ({})", self.what(), self.fix())
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

    fn what(&self) -> String {
        let file = self.file().display();
        match self {
            Self::Read { reason, .. } => {
                format!("cannot read {file}: {reason}")
            }
            Self::Parse { reason, .. } => {
                format!("{file} is not valid TOML: {reason}")
            }
            Self::UnknownKey { key, .. } => {
                format!("unknown key `{key}` in {file}")
            }
            Self::Invalid { key, value, .. } => {
                format!("`{key} = {value}` in {file} is invalid")
            }
        }
    }

    fn fix(&self) -> String {
        match self {
            Self::Read { .. } => "check the file".to_owned(),
            Self::Parse { .. } => "fix its syntax".to_owned(),
            Self::UnknownKey { .. } => "remove or respell it".to_owned(),
            Self::Invalid { accepted, .. } => format!("accepted: {accepted}"),
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

const SPAN_UNITS: &[(&str, Unit)] =
    &[("mo", Unit::Months), ("d", Unit::Days), ("h", Unit::Hours)];

impl Span {
    /// Parse `<n>d`, `<n>h` or `<n>mo`, n >= 1.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let (amount, unit) = split_unit(text, SPAN_UNITS)?;
        Some(Self { amount, unit })
    }

    /// The span in hours; `None` for months.
    fn hours(self) -> Option<u64> {
        let amount = u64::from(self.amount);
        match self.unit {
            Unit::Hours => Some(amount),
            Unit::Days => amount.checked_mul(24),
            Unit::Months => None,
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suffix = SPAN_UNITS
            .iter()
            .find(|&&(_, unit)| unit == self.unit)
            .map_or("", |&(suffix, _)| suffix);
        write!(f, "{}{suffix}", self.amount)
    }
}

const SIZE_UNITS: &[(&str, u64)] =
    &[("KiB", 1 << 10), ("MiB", 1 << 20), ("GiB", 1 << 30)];

/// Parse a size `<n>KiB`, `<n>MiB` or `<n>GiB`, n >= 1, into bytes.
#[must_use]
pub fn parse_size(text: &str) -> Option<u64> {
    let (amount, scale) = split_unit(text, SIZE_UNITS)?;
    u64::from(amount).checked_mul(scale)
}

/// `<digits><suffix>` for the first suffix that matches, digits >= 1.
fn split_unit<T: Copy>(text: &str, units: &[(&str, T)]) -> Option<(u32, T)> {
    units.iter().find_map(|&(suffix, unit)| {
        let digits = text.strip_suffix(suffix)?;
        Some((positive_digits(digits)?, unit))
    })
}

fn positive_digits(digits: &str) -> Option<u32> {
    if !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u32>().ok().filter(|n| *n > 0)
}

/// A `partition.by` dimension (src/config C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dimension {
    Project,
    Time,
    Ide,
}

const DIMENSIONS: &[(&str, Dimension)] = &[
    ("project", Dimension::Project),
    ("time", Dimension::Time),
    ("ide", Dimension::Ide),
];

/// `cache.on_overflow` (src/cache C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    Warn,
    Shrink,
    Fail,
}

const OVERFLOWS: &[(&str, Overflow)] = &[
    ("warn", Overflow::Warn),
    ("shrink", Overflow::Shrink),
    ("fail", Overflow::Fail),
];

/// `compression.grammar`, the caveman intensity (src/grammar G).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grammar {
    Off,
    Lite,
    Full,
    Ultra,
}

const GRAMMARS: &[(&str, Grammar)] = &[
    ("off", Grammar::Off),
    ("lite", Grammar::Lite),
    ("full", Grammar::Full),
    ("ultra", Grammar::Ultra),
];

/// `embedding.provider` (root C, D18).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Off,
    Ollama,
    Openai,
}

const PROVIDERS: &[(&str, Provider)] = &[
    ("off", Provider::Off),
    ("ollama", Provider::Ollama),
    ("openai", Provider::Openai),
];

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
    home: Option<&Path>,
    project_root: &Path,
) -> Result<Loaded, Error> {
    let source = Source::gather(&files(home, project_root))?;
    let config = Config::build(&source, home)?;
    let (window, bucket) = (config.cache.window, config.partition.time_bucket);
    let warnings = window_warning(window, bucket).into_iter().collect();
    Ok(Loaded { config, warnings })
}

/// A warning when `window` is not a whole multiple of `bucket`
/// (src/config V2); the load still succeeds.
fn window_warning(window: Span, bucket: Span) -> Option<String> {
    if whole_multiple(window, bucket) {
        return None;
    }
    Some(format!(
        "chert: cache.window = \"{window}\" is not a whole multiple of \
         partition.time_bucket = \"{bucket}\" (make the window a whole \
         number of buckets)"
    ))
}

/// Hours and days compare in hours; months only against months, as a
/// month has no fixed length in days.
fn whole_multiple(window: Span, bucket: Span) -> bool {
    match (window.hours(), bucket.hours()) {
        (Some(w), Some(b)) => w.checked_rem(b) == Some(0),
        (None, None) => window.amount.checked_rem(bucket.amount) == Some(0),
        _ => false,
    }
}

/// The user file (when there is a home), then the project file: the
/// later wins (V1).
fn files(home: Option<&Path>, project_root: &Path) -> Vec<PathBuf> {
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
struct Source {
    entries: BTreeMap<String, Entry>,
}

impl Source {
    fn gather(files: &[PathBuf]) -> Result<Self, Error> {
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
    fn get<T>(
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
    fn pick<T: Copy>(
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
            .map(|(key, v)| (format!("{section}.{key}"), v))
            .collect(),
        other => vec![(section, other)],
    });
    pairs.collect()
}

fn name<T: Copy>(value: &Value, names: &[(&str, T)]) -> Option<T> {
    let text = value.as_str()?;
    names.iter().find(|(n, _)| *n == text).map(|&(_, t)| t)
}

fn span(value: &Value) -> Option<Span> {
    value.as_str().and_then(Span::parse)
}

fn size(value: &Value) -> Option<u64> {
    value.as_str().and_then(parse_size)
}

fn positive(value: &Value) -> Option<u32> {
    let n = u32::try_from(value.as_integer()?).ok()?;
    (n > 0).then_some(n)
}

fn alpha(value: &Value) -> Option<f64> {
    let int = || value.as_integer().and_then(|n| i32::try_from(n).ok());
    let a = value.as_float().or_else(|| int().map(f64::from))?;
    (0.0..=1.0).contains(&a).then_some(a)
}

fn strings(value: &Value) -> Option<Vec<String>> {
    let items = value.as_array()?.iter();
    items.map(|v| v.as_str().map(str::to_owned)).collect()
}

/// Distinct dimensions, in order, holding `time` (src/config C).
fn dimensions(value: &Value) -> Option<Vec<Dimension>> {
    let items = value.as_array()?.iter();
    let dims: Vec<Dimension> =
        items.map(|v| name(v, DIMENSIONS)).collect::<Option<_>>()?;
    let mut seen = Vec::new();
    let distinct = dims.iter().all(|d| {
        let new = !seen.contains(d);
        seen.push(*d);
        new
    });
    (distinct && dims.contains(&Dimension::Time)).then_some(dims)
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
        })
    }
}

impl Storage {
    fn build(src: &Source, home: Option<&Path>) -> Result<Self, Error> {
        let set = src
            .get("storage.data_dir", PATH, |v| v.as_str().map(PathBuf::from))?;
        let default = || home.map(|h| h.join(".local/share/chert"));
        Ok(Self {
            data_dir: set.or_else(default),
        })
    }
}

/// Root C D19 defaults.
const TIME_BUCKET: Span = Span {
    amount: 7,
    unit: Unit::Days,
};
const WINDOW: Span = Span {
    amount: 28,
    unit: Unit::Days,
};
const MAX_BYTES: u64 = 256 << 20;

impl Partition {
    fn build(src: &Source) -> Result<Self, Error> {
        let by = src.get("partition.by", BY_FORMS, dimensions)?;
        let bucket = src.get("partition.time_bucket", SPAN_FORMS, span)?;
        let shards = src.get("partition.shards", POSITIVE, positive)?;
        Ok(Self {
            by: by.unwrap_or_else(|| vec![Dimension::Project, Dimension::Time]),
            time_bucket: bucket.unwrap_or(TIME_BUCKET),
            shards: shards.unwrap_or(1),
        })
    }
}

impl Cache {
    fn build(src: &Source) -> Result<Self, Error> {
        let window = src.get("cache.window", SPAN_FORMS, span)?;
        let max_bytes = src.get("cache.max_bytes", SIZE_FORMS, size)?;
        let on_overflow = src.pick("cache.on_overflow", OVERFLOWS)?;
        Ok(Self {
            window: window.unwrap_or(WINDOW),
            max_bytes: max_bytes.unwrap_or(MAX_BYTES),
            on_overflow: on_overflow.unwrap_or(Overflow::Warn),
        })
    }
}

impl Compression {
    fn build(src: &Source) -> Result<Self, Error> {
        Ok(Self {
            grammar: src.pick("compression.grammar", GRAMMARS)?,
            zstd: src.get("compression.zstd", BOOL, Value::as_bool)?,
        })
    }
}

impl Search {
    fn build(src: &Source) -> Result<Self, Error> {
        Ok(Self {
            alpha: src.get("search.alpha", ALPHA, alpha)?,
            default_limit: src.get(
                "search.default_limit",
                POSITIVE,
                positive,
            )?,
        })
    }
}

impl Embedding {
    fn build(src: &Source) -> Result<Self, Error> {
        let provider = src.pick("embedding.provider", PROVIDERS)?;
        Ok(Self {
            provider: provider.unwrap_or(Provider::Off),
        })
    }
}

impl Privacy {
    fn build(src: &Source) -> Result<Self, Error> {
        let exclude = src.get("privacy.exclude", PATHS, strings)?;
        Ok(Self {
            exclude: exclude.unwrap_or_default(),
        })
    }
}

impl Mining {
    fn build(src: &Source) -> Result<Self, Error> {
        let min = src.get("mining.min_occurrences", POSITIVE, positive)?;
        Ok(Self {
            enabled: src.get("mining.enabled", BOOL, Value::as_bool)?,
            min_occurrences: min,
        })
    }
}
