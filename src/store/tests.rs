//! Store-wide tests, and the scratch store every store test opens.

use std::error::Error as StdError;
use std::fs;
use std::path::PathBuf;

use rusqlite::Connection;

use super::{Appended, Error, Event, Mode, Ref, Store};
use crate::config::{Config, load_from};

pub(super) type Test = Result<(), Box<dyn StdError>>;

/// A scratch dir holding `project/` and the data dir `data/`, removed on
/// drop.
pub(super) struct Scratch(PathBuf);

impl Scratch {
    pub(super) fn new(name: &str) -> Result<Self, Box<dyn StdError>> {
        let pid = std::process::id();
        let dir =
            std::env::temp_dir().join(format!("chert-store-{pid}-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("project"))?;
        Ok(Self(dir))
    }

    pub(super) fn project(&self) -> PathBuf {
        self.0.join("project")
    }

    /// A store for another project root `name` in the same data dir.
    pub(super) fn open_other(
        &self,
        name: &str,
    ) -> Result<Store, Box<dyn StdError>> {
        let root = self.0.join(name);
        fs::create_dir_all(&root)?;
        Ok(Store::open(&self.config()?, &root, Mode::Read)?)
    }

    pub(super) fn data(&self) -> PathBuf {
        self.0.join("data")
    }

    pub(super) fn config(&self) -> Result<Config, Box<dyn StdError>> {
        let mut cfg = load_from(None, &self.project())?.config;
        cfg.storage.data_dir = Some(self.data());
        Ok(cfg)
    }

    pub(super) fn open(&self, mode: Mode) -> Result<Store, Box<dyn StdError>> {
        Ok(Store::open(&self.config()?, &self.project(), mode)?)
    }

    /// A raw connection to `path` under the data dir.
    pub(super) fn sql(
        &self,
        path: &str,
    ) -> Result<Connection, Box<dyn StdError>> {
        Ok(Connection::open(self.data().join(path))?)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 1 week in milliseconds: the default bucket.
pub(super) const WEEK: u64 = 7 * 86_400_000;

pub(super) fn event(ts: u64, identity: Option<&str>, body: &str) -> Event {
    Event {
        ts,
        session: "session-1".into(),
        kind: "post-tool-use".into(),
        ide: "claude-code".into(),
        identity: identity.map(str::to_owned),
        body: body.into(),
    }
}

/// The ref of a stored append.
pub(super) fn stored(appended: Appended) -> Result<Ref, Box<dyn StdError>> {
    match appended {
        Appended::Stored(r) => Ok(r),
        Appended::Spooled => Err("spooled".into()),
    }
}

/// `chunks/<project>/<bucket>.db` of `store`.
pub(super) fn chunk(store: &Store, bucket: &str) -> String {
    format!("chunks/{}/{bucket}.db", store.project())
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

#[test]
fn the_project_is_the_id_of_its_root() -> Test {
    let s = Scratch::new("project")?;
    let store = s.open(Mode::Write)?;
    assert_eq!(store.project(), super::project_id(&s.project()));
    assert!(store.project().starts_with("project-"));
    Ok(())
}
