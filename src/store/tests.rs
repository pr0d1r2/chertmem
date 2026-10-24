use std::error::Error as StdError;
use std::fs;
use std::path::PathBuf;

use super::{Error, Mode, Store};
use crate::config::{Config, load_from};

type Test = Result<(), Box<dyn StdError>>;

/// A scratch dir holding `project/` and the data dir `data/`, removed on
/// drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Result<Self, Box<dyn StdError>> {
        let pid = std::process::id();
        let dir =
            std::env::temp_dir().join(format!("chert-store-{pid}-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("project"))?;
        Ok(Self(dir))
    }

    fn project(&self) -> PathBuf {
        self.0.join("project")
    }

    fn data(&self) -> PathBuf {
        self.0.join("data")
    }

    fn config(&self) -> Result<Config, Box<dyn StdError>> {
        let mut cfg = load_from(None, &self.project())?.config;
        cfg.storage.data_dir = Some(self.data());
        Ok(cfg)
    }

    fn open(&self, mode: Mode) -> Result<Store, Box<dyn StdError>> {
        Ok(Store::open(&self.config()?, &self.project(), mode)?)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn reading_open_lays_catalog_chunks_and_spool() -> Test {
    let s = Scratch::new("layout")?;
    drop(s.open(Mode::Read)?);
    for name in ["catalog.db", "chunks", "spool"] {
        assert!(s.data().join(name).exists(), "{name}");
    }
    Ok(())
}

#[test]
fn write_only_open_touches_nothing_yet() -> Test {
    let s = Scratch::new("lazy")?;
    drop(s.open(Mode::Write)?);
    assert!(!s.data().exists());
    Ok(())
}

#[test]
fn no_data_dir_is_an_error() -> Test {
    let s = Scratch::new("nodir")?;
    let mut cfg = s.config()?;
    cfg.storage.data_dir = None;
    let e = Store::open(&cfg, &s.project(), Mode::Read).err();
    assert_eq!(e, Some(Error::NoDataDir));
    Ok(())
}

#[test]
fn an_unusable_data_dir_is_an_io_error() -> Test {
    let s = Scratch::new("blocked")?;
    fs::write(s.data(), "a file, not a dir")?;
    let e = Store::open(&s.config()?, &s.project(), Mode::Read).err();
    assert!(matches!(e, Some(Error::Io { .. })), "{e:?}");
    Ok(())
}
