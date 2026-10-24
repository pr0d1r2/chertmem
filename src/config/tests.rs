use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use super::{
    Config, Dimension, Error as ConfigError, Grammar, Loaded, Overflow,
    Provider, Span, Unit, load_from, parse_size,
};

type Test = Result<(), Box<dyn Error>>;

/// A scratch dir holding `home/` and `project/`, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Result<Self, Box<dyn Error>> {
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("chert-cfg-{pid}-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home"))?;
        fs::create_dir_all(dir.join("project"))?;
        Ok(Self(dir))
    }

    fn home(&self) -> PathBuf {
        self.0.join("home")
    }

    fn project(&self) -> PathBuf {
        self.0.join("project")
    }

    fn user_file(&self) -> PathBuf {
        self.home().join(".config/chert/chert.toml")
    }

    fn project_file(&self) -> PathBuf {
        self.project().join("chert.toml")
    }

    fn load(&self) -> Result<Loaded, ConfigError> {
        load_from(Some(&self.home()), &self.project())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn put(file: &Path, text: &str) -> Result<(), Box<dyn Error>> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(file, text)?;
    Ok(())
}

/// The config a project file holding `text` gives.
fn project_only(name: &str, text: &str) -> Result<Config, Box<dyn Error>> {
    let s = Scratch::new(name)?;
    put(&s.project_file(), text)?;
    Ok(s.load()?.config)
}

/// The error a project file holding `text` gives.
fn project_error(
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

const fn days(amount: u32) -> Span {
    Span {
        amount,
        unit: Unit::Days,
    }
}

const fn hours(amount: u32) -> Span {
    Span {
        amount,
        unit: Unit::Hours,
    }
}

const fn months(amount: u32) -> Span {
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

#[test]
fn keys_without_a_root_default_stay_unset() -> Test {
    let c = Scratch::new("unset")?.load()?.config;
    let (z, s, m) = (c.compression, c.search, c.mining);
    assert_eq!((z.grammar, z.zstd), (None, None));
    assert_eq!((s.alpha, s.default_limit), (None, None));
    assert_eq!((m.enabled, m.min_occurrences), (None, None));
    Ok(())
}

#[test]
fn data_dir_defaults_under_home() -> Test {
    let s = Scratch::new("datadir")?;
    let dir = s.load()?.config.storage.data_dir;
    assert_eq!(dir, Some(s.home().join(".local/share/chert")));
    let none = load_from(None, &s.project())?.config.storage.data_dir;
    assert_eq!(none, None);
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
fn user_file_alone_applies() -> Test {
    let s = Scratch::new("useronly")?;
    put(&s.user_file(), "embedding.provider = \"ollama\"\n")?;
    assert_eq!(s.load()?.config.embedding.provider, Provider::Ollama);
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
fn partition_keys_parse() -> Test {
    let text = "storage.data_dir = \"/d\"\n\
        partition.by = [\"time\", \"ide\", \"project\"]\n\
        partition.time_bucket = \"12h\"\npartition.shards = 3\n";
    let c = project_only("partition", text)?;
    assert_eq!(c.storage.data_dir, Some(PathBuf::from("/d")));
    let by = vec![Dimension::Time, Dimension::Ide, Dimension::Project];
    let p = c.partition;
    assert_eq!((p.by, p.time_bucket, p.shards), (by, hours(12), 3));
    Ok(())
}

#[test]
fn cache_keys_parse() -> Test {
    let text = "cache.window = \"2mo\"\ncache.max_bytes = \"2GiB\"\n\
        cache.on_overflow = \"shrink\"\n";
    let c = project_only("cache", text)?;
    let k = c.cache;
    let cache = (months(2), 2 << 30, Overflow::Shrink);
    assert_eq!((k.window, k.max_bytes, k.on_overflow), cache);
    Ok(())
}

#[test]
fn compression_and_search_keys_parse() -> Test {
    let text = "compression.grammar = \"ultra\"\ncompression.zstd = false\n\
        search.alpha = 0.25\nsearch.default_limit = 20\n";
    let c = project_only("search", text)?;
    let (z, s) = (c.compression, c.search);
    assert_eq!((z.grammar, z.zstd), (Some(Grammar::Ultra), Some(false)));
    assert_eq!((s.alpha, s.default_limit), (Some(0.25), Some(20)));
    Ok(())
}

#[test]
fn embedding_privacy_and_mining_keys_parse() -> Test {
    let text = "embedding.provider = \"openai\"\n\
        privacy.exclude = [\".env\", \"secrets/\"]\n\
        mining.enabled = false\nmining.min_occurrences = 5\n";
    let c = project_only("mining", text)?;
    assert_eq!(c.embedding.provider, Provider::Openai);
    assert_eq!(c.privacy.exclude, vec![".env", "secrets/"]);
    let m = c.mining;
    assert_eq!((m.enabled, m.min_occurrences), (Some(false), Some(5)));
    Ok(())
}

#[test]
fn remaining_names_parse() -> Test {
    let text = "cache.on_overflow = \"fail\"\ncompression.grammar = \"off\"\n\
        search.alpha = 1\n";
    let c = project_only("names", text)?;
    assert_eq!(c.cache.on_overflow, Overflow::Fail);
    assert_eq!(c.compression.grammar, Some(Grammar::Off));
    assert_eq!(c.search.alpha, Some(1.0));
    Ok(())
}

#[test]
fn sizes_parse_kib_mib_gib() {
    assert_eq!(parse_size("4KiB"), Some(4096));
    assert_eq!(parse_size("1MiB"), Some(1 << 20));
    assert_eq!(parse_size("3GiB"), Some(3 << 30));
    for bad in ["4kb", "MiB", "0MiB", "-1MiB", "1.5GiB", "1 MiB", "1TiB"] {
        assert_eq!(parse_size(bad), None, "{bad}");
    }
}

#[test]
fn durations_parse_d_h_mo() {
    assert_eq!(Span::parse("7d"), Some(days(7)));
    assert_eq!(Span::parse("36h"), Some(hours(36)));
    assert_eq!(Span::parse("1mo"), Some(months(1)));
    for bad in ["7w", "0d", "d", "-3d", "+3d", "3 d", "1m", "1.5d", ""] {
        assert_eq!(Span::parse(bad), None, "{bad}");
    }
}

#[test]
fn spans_display_as_written() {
    assert_eq!(days(7).to_string(), "7d");
    assert_eq!(months(2).to_string(), "2mo");
    assert_eq!(hours(5).to_string(), "5h");
}

#[test]
fn unknown_key_names_key_and_file() -> Test {
    let s = Scratch::new("unknown")?;
    put(&s.user_file(), "[cache]\nwindoww = \"7d\"\n")?;
    let want = ConfigError::UnknownKey {
        file: s.user_file(),
        key: "cache.windoww".to_owned(),
    };
    assert_eq!(s.load(), Err(want));
    Ok(())
}

const UNKNOWN: &[(&str, &str)] = &[
    ("colour = 1\n", "colour"),
    ("[nope]\na = 1\n", "nope.a"),
    ("cache = 1\n", "cache"),
];

#[test]
fn unknown_sections_are_errors() -> Test {
    for (i, &(text, key)) in UNKNOWN.iter().enumerate() {
        let e = project_error(&format!("section{i}"), text)?;
        let want = ConfigError::UnknownKey {
            file: e.file().to_path_buf(),
            key: key.to_owned(),
        };
        assert_eq!(e, want, "{text:?}");
    }
    Ok(())
}

#[test]
fn table_where_value_belongs_is_invalid() -> Test {
    let e = project_error("tablevalue", "[cache.window]\na = 1\n")?;
    let invalid = matches!(e, ConfigError::Invalid { ref key, .. } if key == "cache.window");
    assert!(invalid, "{e}");
    Ok(())
}

#[test]
fn invalid_value_names_file_key_value_and_forms() -> Test {
    let s = Scratch::new("invalid")?;
    put(&s.project_file(), "[cache]\nwindow = \"4w\"\n")?;
    let want = ConfigError::Invalid {
        file: s.project_file(),
        key: "cache.window".to_owned(),
        value: "\"4w\"".to_owned(),
        accepted: "<n>d, <n>h or <n>mo with n >= 1".to_owned(),
    };
    assert_eq!(s.load(), Err(want));
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
fn broken_toml_is_a_parse_error() -> Test {
    let e = project_error("parse", "[cache\n")?;
    assert!(matches!(e, ConfigError::Parse { .. }), "{e}");
    Ok(())
}

#[test]
fn unreadable_file_is_a_read_error() -> Test {
    let s = Scratch::new("read")?;
    fs::create_dir_all(s.project_file())?;
    let e = s.load();
    assert!(matches!(e, Err(ConfigError::Read { .. })), "{e:?}");
    Ok(())
}

fn invalid_window() -> ConfigError {
    ConfigError::Invalid {
        file: PathBuf::from("/p/chert.toml"),
        key: "cache.window".into(),
        value: "\"4w\"".into(),
        accepted: "<n>d".into(),
    }
}

fn every_error() -> [ConfigError; 4] {
    let p = || PathBuf::from("/p/chert.toml");
    let (file, reason, key) = (p(), "denied".to_owned(), "a.b".to_owned());
    let read = ConfigError::Read { file, reason };
    let (file, reason) = (p(), "line 1".to_owned());
    let parse = ConfigError::Parse { file, reason };
    let file = p();
    let unknown = ConfigError::UnknownKey { file, key };
    [read, parse, unknown, invalid_window()]
}

#[test]
fn messages_follow_chert_what_fix() {
    for e in every_error() {
        let text = e.to_string();
        assert!(text.starts_with("chert: "), "{text}");
        assert!(
            text.contains("/p/chert.toml") && text.ends_with(')'),
            "{text}"
        );
    }
}

#[test]
fn invalid_message_names_key_value_and_forms() {
    let text = invalid_window().to_string();
    assert!(text.contains("cache.window = \"4w\""), "{text}");
    assert!(text.contains("accepted: <n>d"), "{text}");
}
