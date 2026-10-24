//! Search tests, and the scratch data dir every search test opens.

use std::error::Error as StdError;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{Config, load_from};
use crate::store::{Event, Mode, Store};

pub(super) type Test = Result<(), Box<dyn StdError>>;

pub(super) const DAY: u64 = 86_400_000;
/// 2026-01-01T00:00:00Z: inside 1 `120mo` bucket, 2020-01 to 2030-01.
pub(super) const BASE: u64 = 1_767_225_600_000;

/// A scratch dir holding `project/` and the data dir `data/`, removed on
/// drop.
pub(super) struct Scratch(PathBuf);

impl Scratch {
    pub(super) fn new(name: &str) -> Result<Self, Box<dyn StdError>> {
        let pid = std::process::id();
        let dir =
            std::env::temp_dir().join(format!("chert-search-{pid}-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("project"))?;
        Ok(Self(dir))
    }

    pub(super) fn root(&self) -> PathBuf {
        self.0.join("project")
    }

    pub(super) fn data(&self) -> PathBuf {
        self.0.join("data")
    }

    /// No user file, data in `data/`.
    pub(super) fn config(&self) -> Result<Config, Box<dyn StdError>> {
        config_at(&self.0)
    }

    pub(super) fn open(
        &self,
        cfg: &Config,
    ) -> Result<Store, Box<dyn StdError>> {
        Ok(Store::open(cfg, &self.root(), Mode::Read)?)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn config_at(base: &Path) -> Result<Config, Box<dyn StdError>> {
    let mut cfg = load_from(None, &base.join("project"))?.config;
    cfg.storage.data_dir = Some(base.join("data"));
    Ok(cfg)
}

pub(super) fn event(ts: u64, session: &str, body: &str) -> Event {
    Event {
        ts,
        session: session.into(),
        kind: "post-tool-use".into(),
        ide: "claude-code".into(),
        identity: None,
        body: body.into(),
    }
}
