//! Import-wide tests, and the scratch project and data dir every import
//! test opens.

use std::error::Error as StdError;
use std::fs;
use std::path::PathBuf;

use crate::config::{Config, load_from};
use crate::store::{Event, Mode, Store};

pub(super) type Test = Result<(), Box<dyn StdError>>;
pub(super) type Res<T> = Result<T, Box<dyn StdError>>;

/// Milliseconds of 2026-10-24, the time test events happen around.
pub(super) const NOW: u64 = 1_792_800_000_000;

/// A scratch dir holding `project/` and the data dir `data/`, removed on
/// drop; the config points at that data dir.
pub(super) struct Scratch {
    dir: PathBuf,
    pub(super) config: Config,
}

impl Scratch {
    pub(super) fn new(name: &str) -> Res<Self> {
        let pid = std::process::id();
        let dir =
            std::env::temp_dir().join(format!("chert-import-{pid}-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("project"))?;
        let mut config = load_from(None, &dir.join("project"))?.config;
        config.storage.data_dir = Some(dir.join("data"));
        Ok(Self { dir, config })
    }

    pub(super) fn root(&self) -> PathBuf {
        self.dir.join("project")
    }

    /// A path inside the scratch dir, outside the data dir.
    pub(super) fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    pub(super) fn open(&self, mode: Mode) -> Res<Store> {
        Ok(Store::open(&self.config, &self.root(), mode)?)
    }

    pub(super) fn store(&self) -> Res<Store> {
        self.open(Mode::Read)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// An event `at` ms after [`NOW`] in session `s1`.
pub(super) fn event(at: u64, body: &str, identity: Option<&str>) -> Event {
    Event {
        ts: NOW.saturating_add(at),
        session: "s1".to_owned(),
        kind: "post-tool-use".to_owned(),
        ide: "claude-code".to_owned(),
        identity: identity.map(str::to_owned),
        body: body.to_owned(),
    }
}

#[test]
fn scratch_dirs_are_removed_on_drop() -> Test {
    let path = {
        let s = Scratch::new("drop")?;
        s.store()?;
        s.path("")
    };
    assert!(!path.exists());
    Ok(())
}
