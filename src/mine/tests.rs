//! Mine-wide tests, the observations the signal tests mine, and the
//! scratch project and data dir the run tests fill through `src/hook`.

use std::error::Error as StdError;
use std::fs;
use std::path::PathBuf;

use serde_json::{Map, Value, json};

use super::Kind;
use crate::config::{Config, load_from};
use crate::grammar::{Intensity, compress};
use crate::hook::{self, Ctx, Ide};
use crate::store::{Mode, Observation, Ref, Store};

pub(super) type Test = Result<(), Box<dyn StdError>>;
pub(super) type Res<T> = Result<T, Box<dyn StdError>>;

/// The recorded Claude Code `PostToolUseFailure` payload (src/hook R5).
const FAILURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/hook/fixtures/claude-code/post-tool-use-failure.json"
);

/// Milliseconds of 2026-10-24, the time test events happen around.
pub(super) const NOW: u64 = 1_792_800_000_000;

/// A scratch dir holding `project/` and the data dir `data/`, removed on
/// drop; the config points at that data dir.
pub(super) struct Scratch {
    pub(super) dir: PathBuf,
    pub(super) config: Config,
}

impl Scratch {
    pub(super) fn new(name: &str) -> Res<Self> {
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("chert-mine-{pid}-{name}"));
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

    pub(super) fn store(&self) -> Res<Store> {
        Ok(Store::open(&self.config, &self.root(), Mode::Read)?)
    }

    /// A Claude Code `post-tool-use` of `tool` with `input`, `at` ms
    /// after [`NOW`] in `session`, stored by `src/hook`.
    pub(super) fn tool(
        &self,
        at: u64,
        session: &str,
        tool: (&str, &Value),
    ) -> Test {
        let payload = json!({
            "session_id": session, "tool_name": tool.0, "tool_input": tool.1,
            "tool_response": {"stdout": ""}, "tool_use_id": format!("toolu_{at}"),
        });
        let now = NOW.checked_add(at).ok_or("time out of range")?;
        let stdin = payload.to_string();
        self.hook(hook::Kind::PostToolUse, now, stdin.as_bytes())
    }

    /// The recorded `post-tool-use-failure` (src/hook R5) as a call of
    /// `call.0` with input `call.1` failing with error `call.2`, `at` ms
    /// after [`NOW`] in `session`, stored by `src/hook`.
    pub(super) fn failure(
        &self,
        at: u64,
        session: &str,
        call: (&str, &Value, &str),
    ) -> Test {
        let mut payload: Value = serde_json::from_slice(&fs::read(FAILURE)?)?;
        let fields: Map<String, Value> = serde_json::from_value(json!({
            "session_id": session, "tool_name": call.0, "tool_input": call.1,
            "error": call.2, "tool_use_id": format!("toolu_{at}"),
        }))?;
        let obj = payload.as_object_mut().ok_or("fixture not an object")?;
        obj.extend(fields);
        let now = NOW.checked_add(at).ok_or("time out of range")?;
        let stdin = payload.to_string();
        self.hook(hook::Kind::PostToolUseFailure, now, stdin.as_bytes())
    }

    /// Run the `kind` hook on `stdin` at `now`.
    fn hook(&self, kind: hook::Kind, now: u64, stdin: &[u8]) -> Test {
        let root = self.root();
        let ide = Ide::ClaudeCode;
        let config = &self.config;
        let ctx = Ctx {
            config,
            root: &root,
            now,
            ide,
        };
        let done = hook::run(kind, stdin, &ctx);
        done.note.map_or(Ok(()), |n| Err(n.into()))
    }

    /// A Bash call of `command`, as [`Self::tool`].
    pub(super) fn bash(&self, at: u64, session: &str, command: &str) -> Test {
        self.tool(at, session, ("Bash", &json!({ "command": command })))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// A `post-tool-use` row `n` ms after [`NOW`] in `session`, with the
/// body `src/hook` stores for tool `name` with `input`; its ref is `n`
/// in 8 digits.
pub(super) fn row(
    n: u64,
    session: &str,
    name: &str,
    input: &str,
) -> Res<Observation> {
    let raw = format!("{name}\n{input}\n{{\"stdout\":\"\"}}");
    observation(n, session, &raw)
}

/// A `post-tool-use-failure` row as [`row`], with the body `src/hook`
/// stores for tool `call.0` with input `call.1` failing with `error`.
pub(super) fn failed(
    n: u64,
    session: &str,
    call: (&str, &str),
    error: &str,
) -> Res<Observation> {
    let raw = format!("{}\n{}\n{error}", call.0, call.1);
    let mut failed = observation(n, session, &raw)?;
    failed.kind = "post-tool-use-failure".to_owned();
    Ok(failed)
}

/// A `post-tool-use` row as [`row`], its body `raw` compressed.
fn observation(n: u64, session: &str, raw: &str) -> Res<Observation> {
    Ok(Observation {
        reference: reference(n)?,
        id: format!("{n:026}"),
        project: "project".to_owned(),
        ts: NOW.checked_add(n).ok_or("time out of range")?,
        session: session.to_owned(),
        ide: "claude-code".to_owned(),
        kind: "post-tool-use".to_owned(),
        identity: None,
        body: compress(raw, Intensity::Full),
    })
}

/// The ref of [`row`] `n`.
pub(super) fn reference(n: u64) -> Res<Ref> {
    Ok(Ref::parse(&format!("chert:{n:08}"))?)
}

/// A Bash call of `command` as row `n` in `session`.
pub(super) fn bash(n: u64, session: &str, command: &str) -> Res<Observation> {
    let input = serde_json::json!({ "command": command }).to_string();
    row(n, session, "Bash", &input)
}

#[test]
fn every_kind_round_trips_its_spelling() {
    let all = [
        Kind::Failure,
        Kind::Sequence,
        Kind::HotFile,
        Kind::TokenSink,
    ];
    for kind in all {
        assert_eq!(Kind::parse(kind.as_str()), Some(kind));
    }
    assert_eq!(Kind::parse("error"), None);
}

/// The public entry mines rows read any way, as a hot partition is.
#[test]
fn findings_mines_rows_given_directly() -> Test {
    let rows = [bash(1, "s", "ls")?, bash(2, "s", "pwd")?];
    let found = super::findings(&rows, 1);
    let seqs = found.iter().filter(|f| f.kind == Kind::Sequence);
    let kinds: Vec<_> = seqs.map(|f| (f.kind, f.count)).collect();
    assert_eq!(kinds, [(Kind::Sequence, 1)]);
    assert!(super::findings(&rows, 2).is_empty());
    Ok(())
}
