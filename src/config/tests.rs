use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use super::{
    Config, Dimension, Error as ConfigError, Grammar, Loaded, Overflow,
    Provider, Span, Unit, Zstd, load_from,
};

/// Shared by every test module under src/config.
pub(super) type Test = Result<(), Box<dyn Error>>;

/// A scratch dir holding `home/` and `project/`, removed on drop.
pub(super) struct Scratch(PathBuf);

impl Scratch {
    pub(super) fn new(name: &str) -> Result<Self, Box<dyn Error>> {
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("chert-cfg-{pid}-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home"))?;
        fs::create_dir_all(dir.join("project"))?;
        Ok(Self(dir))
    }

    pub(super) fn home(&self) -> PathBuf {
        self.0.join("home")
    }

    pub(super) fn project(&self) -> PathBuf {
        self.0.join("project")
    }

    pub(super) fn user_file(&self) -> PathBuf {
        self.home().join(".config/chert/chert.toml")
    }

    pub(super) fn project_file(&self) -> PathBuf {
        self.project().join("chert.toml")
    }

    pub(super) fn load(&self) -> Result<Loaded, ConfigError> {
        load_from(Some(&self.home()), &self.project())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) fn put(file: &Path, text: &str) -> Result<(), Box<dyn Error>> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(file, text)?;
    Ok(())
}

/// The config a project file holding `text` gives.
pub(super) fn project_only(
    name: &str,
    text: &str,
) -> Result<Config, Box<dyn Error>> {
    let s = Scratch::new(name)?;
    put(&s.project_file(), text)?;
    Ok(s.load()?.config)
}

/// The error a project file holding `text` gives.
pub(super) fn project_error(
    name: &str,
    text: &str,
) -> Result<ConfigError, Box<dyn Error>> {
    let s = Scratch::new(name)?;
    put(&s.project_file(), text)?;
    match s.load() {
        Ok(_) => Err(format!("{text:?} loaded").into()),
        Err(e) => Ok(e),
    }
}

pub(super) const fn days(amount: u32) -> Span {
    Span {
        amount,
        unit: Unit::Days,
    }
}

pub(super) const fn hours(amount: u32) -> Span {
    Span {
        amount,
        unit: Unit::Hours,
    }
}

pub(super) const fn months(amount: u32) -> Span {
    Span {
        amount,
        unit: Unit::Months,
    }
}

const MIB_256: u64 = 256 * 1024 * 1024;

#[test]
fn no_files_give_root_c_defaults() -> Test {
    let c = Scratch::new("defaults")?.load()?.config;
    let p = c.partition;
    let by = vec![Dimension::Project, Dimension::Time];
    assert_eq!((p.by, p.time_bucket, p.shards), (by, days(7), 1));
    let k = c.cache;
    let cache = (days(28), MIB_256, Overflow::Warn);
    assert_eq!((k.window, k.max_bytes, k.on_overflow), cache);
    assert_eq!(c.embedding.provider, Provider::Off);
    assert!(c.privacy.exclude.is_empty());
    Ok(())
}

pub(super) const ZSTD_DEFAULT: Zstd = Zstd {
    level: 3,
    dictionary: true,
};

#[test]
fn remaining_keys_default_per_root_c() -> Test {
    let c = Scratch::new("unset")?.load()?.config;
    let (z, s, m) = (c.compression, c.search, c.mining);
    assert_eq!((z.grammar, z.zstd), (Grammar::Full, ZSTD_DEFAULT));
    assert_eq!((s.alpha, s.default_limit), (0.5, 10));
    assert_eq!((m.enabled, m.min_occurrences), (true, 3));
    Ok(())
}

#[test]
fn project_overrides_user_per_key() -> Test {
    let s = Scratch::new("merge")?;
    let user = "[cache]\nwindow = \"14d\"\n[partition]\nshards = 2\n";
    put(&s.user_file(), user)?;
    put(&s.project_file(), "[cache]\nwindow = \"21d\"\n")?;
    let c = s.load()?.config;
    assert_eq!(c.cache.window, days(21));
    assert_eq!(c.partition.shards, 2);
    assert_eq!(c.cache.max_bytes, MIB_256);
    Ok(())
}

#[test]
fn no_home_skips_the_user_file() -> Test {
    let s = Scratch::new("nohome")?;
    put(&s.user_file(), "[partition]\nshards = 4\n")?;
    let c = load_from(None, &s.project())?.config;
    assert_eq!(c.partition.shards, 1);
    Ok(())
}

#[test]
fn remaining_names_parse() -> Test {
    let text = "cache.on_overflow = \"fail\"\ncompression.grammar = \"off\"\n\
        search.alpha = 1\n";
    let c = project_only("names", text)?;
    assert_eq!(c.cache.on_overflow, Overflow::Fail);
    assert_eq!(c.compression.grammar, Grammar::Off);
    assert!(c.search.alpha.total_cmp(&1.0).is_eq());
    Ok(())
}

const BAD: &[&str] = &[
    "partition.by = [\"project\"]",
    "partition.by = [\"time\", \"time\"]",
    "partition.by = [\"time\", \"host\"]",
    "partition.by = \"time\"",
    "partition.time_bucket = 7",
    "partition.shards = 0",
    "partition.shards = -2",
    "cache.max_bytes = \"256MB\"",
    "cache.on_overflow = \"drop\"",
    "compression.grammar = \"max\"",
    "compression.zstd = \"yes\"",
    "compression.zstd.level = 0",
    "compression.zstd.level = 23",
    "compression.zstd.level = \"3\"",
    "compression.zstd = { level = 1.5 }",
    "compression.zstd = { dictionary = 1 }",
    "search.alpha = 1.5",
    "search.alpha = -0.1",
    "search.default_limit = 0",
    "embedding.provider = \"local\"",
    "privacy.exclude = [1]",
    "storage.data_dir = 3",
    "mining.enabled = 1",
    "mining.min_occurrences = 0",
];

#[test]
fn bad_values_are_errors_not_defaults() -> Test {
    for (i, line) in BAD.iter().enumerate() {
        let e = project_error(&format!("bad{i}"), &format!("{line}\n"))?;
        assert!(matches!(e, ConfigError::Invalid { .. }), "{line}: {e}");
    }
    Ok(())
}

#[test]
fn defaults_load_without_warnings() -> Test {
    let loaded = Scratch::new("nowarn")?.load()?;
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    Ok(())
}
