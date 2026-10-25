//! Hook-wide tests, the scratch project and data dir every hook test
//! uses, and the recorded-payload fixtures (src/hook I).

use std::error::Error as StdError;
use std::fs;
use std::path::PathBuf;

use super::{Ctx, Ide, Kind, Outcome, mode, run};
use crate::config::{Config, Grammar, load_from};
use crate::grammar::{Intensity, compress};
use crate::store::{Filter, Mode, Observation, Store};

pub(super) type Test = Result<(), Box<dyn StdError>>;
pub(super) type Res<T> = Result<T, Box<dyn StdError>>;

/// Milliseconds of 2026-10-24, the time every test event happens at.
pub(super) const NOW: u64 = 1_792_800_000_000;

/// The session of the first recorded session's fixtures (src/hook R1).
pub(super) const SESSION: &str = "00000000-0000-4000-8000-000000000001";

/// The session of the second recorded session's fixtures.
pub(super) const SESSION_2: &str = "00000000-0000-4000-8000-000000000002";

/// The session of the recorded `post-tool-use-failure`, a session of
/// its own (src/hook T9).
pub(super) const SESSION_FAILURE: &str = "00000000-0000-4000-8000-000000000003";

/// The events recorded in both sessions of R1; the failure was
/// recorded once, later.
pub(super) const TWICE: [Kind; 5] = [
    Kind::SessionStart,
    Kind::UserPromptSubmit,
    Kind::PostToolUse,
    Kind::Stop,
    Kind::SessionEnd,
];

/// The session of `kind`'s first fixture.
pub(super) fn session_of(kind: Kind) -> &'static str {
    if kind == Kind::PostToolUseFailure {
        SESSION_FAILURE
    } else {
        SESSION
    }
}

/// The recorded payload of `kind` from the first session (src/hook I).
pub(super) fn fixture(kind: Kind) -> Res<Vec<u8>> {
    recorded(kind, "")
}

/// The recorded payload of `kind` from the second session.
pub(super) fn fixture_2(kind: Kind) -> Res<Vec<u8>> {
    recorded(kind, ".2")
}

/// `tests/hook/fixtures/claude-code/<event><suffix>.json`.
fn recorded(kind: Kind, suffix: &str) -> Res<Vec<u8>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let name = format!("{}{suffix}.json", kind.as_str());
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

/// Fixture `stdin` of `kind` in a fresh store: exactly 1 observation,
/// of `session`, at [`NOW`] (V3).
fn stores_one(kind: Kind, stdin: &[u8], session: &str) -> Test {
    let scratch = Scratch::new(&format!("one-{}-{session}", kind.as_str()))?;
    let out = scratch.run(kind, stdin);
    assert_eq!((out.exit, &out.note), (0, &None), "{kind:?}");
    let found = scratch.observations()?;
    assert_eq!(found.len(), 1, "{kind:?}");
    let obs = found.first().ok_or("no observation")?;
    assert_eq!(obs.kind, kind.as_str());
    assert_eq!(
        (obs.session.as_str(), obs.ide.as_str()),
        (session, "claude-code")
    );
    assert_eq!(obs.ts, NOW);
    Ok(())
}

#[test]
fn every_fixture_stores_exactly_one_observation() -> Test {
    for kind in Kind::ALL {
        stores_one(kind, &fixture(kind)?, session_of(kind))?;
    }
    for kind in TWICE {
        stores_one(kind, &fixture_2(kind)?, SESSION_2)?;
    }
    Ok(())
}

/// The `tool_use_id` of the recorded failed call.
const TOOL_FAILURE: &str = "toolu_01FIXTURE000000000000003";

/// A failed call is its own kind, its body the tool name, input and
/// error, redacted and compressed like any body (T9, V1).
#[test]
fn a_failed_tool_call_stores_its_error_as_its_own_kind() -> Test {
    let scratch = Scratch::new("failure")?;
    let kind = Kind::PostToolUseFailure;
    let out = scratch.run(kind, &fixture(kind)?);
    assert_eq!((out.exit, out.stdout, out.note), (0, None, None));
    let found = scratch.observations()?;
    let obs = found.first().ok_or("no observation")?;
    let input = r#"{"command":"false","description":"Run the false command"}"#;
    let body = format!("Bash\n{input}\nExit code 1");
    let grammar = scratch.config.compression.grammar.intensity();
    assert_eq!(obs.kind, "post-tool-use-failure");
    assert_eq!(obs.identity.as_deref(), Some(TOOL_FAILURE));
    assert_eq!(obs.body, compress(&body, grammar));
    Ok(())
}

#[test]
fn a_failure_resent_is_stored_once() -> Test {
    let scratch = Scratch::new("failure-twice")?;
    let kind = Kind::PostToolUseFailure;
    scratch.run(kind, &fixture(kind)?);
    scratch.run_at(kind, &fixture(kind)?, NOW + 5);
    assert_eq!(scratch.observations()?.len(), 1);
    Ok(())
}

#[test]
fn a_failure_error_loses_its_private_spans() -> Test {
    let scratch = Scratch::new("failure-private")?;
    let stdin = br#"{"session_id":"s","tool_name":"Bash",
        "error":"denied <private>token=hunter2</private>"}"#;
    scratch.run(Kind::PostToolUseFailure, stdin);
    let found = scratch.observations()?;
    let body = found.first().map(|o| o.body.clone()).ok_or("none")?;
    assert!(!body.contains("hunter2"), "{body}");
    assert!(body.contains("denied"), "{body}");
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
    assert_eq!(id.as_deref(), Some(TOOL_USE));
    Ok(())
}

/// The `tool_use_id` of each recorded session's 1 tool call.
const TOOL_USE: &str = "toolu_01FIXTURE000000000000001";
const TOOL_USE_2: &str = "toolu_01FIXTURE000000000000002";

#[test]
fn each_recorded_tool_use_has_its_own_identity() -> Test {
    let scratch = Scratch::new("identity2")?;
    let kind = Kind::PostToolUse;
    scratch.run(kind, &fixture(kind)?);
    // 1 ms later: rows of 1 ms are ordered by random ULID.
    scratch.run_at(kind, &fixture_2(kind)?, NOW + 1);
    let found = scratch.observations()?;
    let ids: Vec<_> = found.iter().map(|o| o.identity.as_deref()).collect();
    assert_eq!(ids, [Some(TOOL_USE), Some(TOOL_USE_2)]);
    Ok(())
}

/// Every event of 1 recorded turn shares its `prompt_id`, so it is no
/// identity (root V16, src/hook T5): each event is kept, repeats too.
#[test]
fn a_shared_prompt_id_dedupes_nothing() -> Test {
    let scratch = Scratch::new("promptid")?;
    let turn = [Kind::UserPromptSubmit, Kind::Stop, Kind::SessionEnd];
    for kind in turn.into_iter().chain(turn) {
        scratch.run(kind, &fixture(kind)?);
    }
    let found = scratch.observations()?;
    assert_eq!(found.len(), 6);
    assert!(found.iter().all(|o| o.identity.is_none()));
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

#[test]
fn only_reading_hooks_open_the_store_for_reading() {
    for kind in Kind::ALL {
        let reads = kind == Kind::SessionEnd;
        let want = if reads { Mode::Read } else { Mode::Write };
        assert_eq!(mode(kind, false), want, "{kind:?}");
        assert_eq!(mode(kind, true), Mode::Read, "{kind:?}");
    }
}
