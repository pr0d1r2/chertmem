//! Hook-wide tests, the scratch project and data dir every hook test
//! uses, and the recorded-payload fixtures (src/hook I).

use std::error::Error as StdError;
use std::fs;
use std::path::PathBuf;

use super::{Ctx, Ide, Kind, Outcome, run};
use crate::config::{Config, Grammar, load_from};
use crate::grammar::{Intensity, compress};
use crate::store::{Filter, Mode, Observation, Store};

pub(super) type Test = Result<(), Box<dyn StdError>>;
pub(super) type Res<T> = Result<T, Box<dyn StdError>>;

/// Milliseconds of 2026-10-24, the time every test event happens at.
pub(super) const NOW: u64 = 1_792_800_000_000;

/// The fixture payload of `kind` (src/hook I).
pub(super) fn fixture(kind: Kind) -> Res<Vec<u8>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let name = format!("{}.json", kind.as_str());
    let path = dir.join("tests/hook/fixtures/claude-code").join(name);
    Ok(fs::read(path)?)
}

/// A scratch dir holding `project/` and the data dir `data/`, removed on
/// drop; the config points at that data dir.
pub(super) struct Scratch {
    dir: PathBuf,
    pub(super) config: Config,
}

impl Scratch {
    pub(super) fn new(name: &str) -> Res<Self> {
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("chert-hook-{pid}-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("project"))?;
        let mut config = load_from(None, &dir.join("project"))?.config;
        config.storage.data_dir = Some(dir.join("data"));
        Ok(Self { dir, config })
    }

    pub(super) fn root(&self) -> PathBuf {
        self.dir.join("project")
    }

    pub(super) fn data(&self) -> PathBuf {
        self.dir.join("data")
    }

    /// Run hook `kind` on `stdin` at `now`.
    pub(super) fn run_at(&self, kind: Kind, stdin: &[u8], now: u64) -> Outcome {
        let root = self.root();
        let ctx = Ctx {
            config: &self.config,
            root: &root,
            now,
            ide: Ide::ClaudeCode,
        };
        run(kind, stdin, &ctx)
    }

    pub(super) fn run(&self, kind: Kind, stdin: &[u8]) -> Outcome {
        self.run_at(kind, stdin, NOW)
    }

    pub(super) fn store(&self) -> Res<Store> {
        Ok(Store::open(&self.config, &self.root(), Mode::Read)?)
    }

    /// Every stored observation, by time then id.
    pub(super) fn observations(&self) -> Res<Vec<Observation>> {
        Ok(self.store()?.observations(&Filter::default())?)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// What V3 compares: the fields an event decides, not the ULID.
fn shape(o: &Observation) -> Shape {
    let o = o.clone();
    (o.session, o.kind, o.ide, o.identity, o.body)
}

#[test]
fn every_fixture_stores_exactly_one_observation() -> Test {
    for kind in Kind::ALL {
        let scratch = Scratch::new(&format!("one-{}", kind.as_str()))?;
        let out = scratch.run(kind, &fixture(kind)?);
        assert_eq!((out.exit, &out.note), (0, &None), "{kind:?}");
        let found = scratch.observations()?;
        assert_eq!(found.len(), 1, "{kind:?}");
        let obs = found.first().ok_or("no observation")?;
        assert_eq!(obs.kind, kind.as_str());
        assert_eq!(
            (obs.session.as_str(), obs.ide.as_str()),
            ("abc123", "claude-code")
        );
        assert_eq!(obs.ts, NOW);
    }
    Ok(())
}

type Shape = (String, String, String, Option<String>, String);

/// The shapes a fresh store holds after the fixture of `kind` ran.
fn shapes_after(kind: Kind, name: &str) -> Res<Vec<Shape>> {
    let scratch = Scratch::new(name)?;
    scratch.run(kind, &fixture(kind)?);
    Ok(scratch.observations()?.iter().map(shape).collect())
}

#[test]
fn every_fixture_stores_the_same_bytes_every_run() -> Test {
    for kind in Kind::ALL {
        let first = shapes_after(kind, &format!("same-{}-0", kind.as_str()))?;
        let again = shapes_after(kind, &format!("same-{}-1", kind.as_str()))?;
        assert_eq!(first, again, "{kind:?}");
    }
    Ok(())
}

#[test]
fn post_tool_use_keeps_its_tool_use_id_as_identity() -> Test {
    let scratch = Scratch::new("identity")?;
    let kind = Kind::PostToolUse;
    scratch.run(kind, &fixture(kind)?);
    scratch.run_at(kind, &fixture(kind)?, NOW + 5);
    let found = scratch.observations()?;
    assert_eq!(found.len(), 1);
    let id = found.first().and_then(|o| o.identity.clone());
    assert_eq!(id.as_deref(), Some("toolu_01ABC123"));
    Ok(())
}

#[test]
fn an_event_without_identity_is_kept_every_time() -> Test {
    let scratch = Scratch::new("noidentity")?;
    let kind = Kind::UserPromptSubmit;
    scratch.run(kind, &fixture(kind)?);
    scratch.run(kind, &fixture(kind)?);
    assert_eq!(scratch.observations()?.len(), 2);
    Ok(())
}

#[test]
fn bodies_are_compressed_at_the_configured_grammar() -> Test {
    let text = br#"{"session_id":"s","prompt":"Please just check the build"}"#;
    let mut bodies = Vec::new();
    for (n, grammar) in [Grammar::Off, Grammar::Full].into_iter().enumerate() {
        let mut scratch = Scratch::new(&format!("grammar-{n}"))?;
        scratch.config.compression.grammar = grammar;
        scratch.run(Kind::UserPromptSubmit, text);
        let found = scratch.observations()?;
        bodies.push(found.first().map(|o| o.body.clone()).ok_or("none")?);
    }
    let full = compress("Please just check the build", Intensity::Full);
    assert_eq!(bodies, ["Please just check the build".to_owned(), full]);
    Ok(())
}
