//! Whole sessions over in-memory stdio, and the scratch data dir every
//! mcp test opens.

use std::error::Error as StdError;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::{Ctx, Error, serve};
use crate::cache::Env;
use crate::config::{Config, load_from};
use crate::store::{Appended, Event, Mode, Ref, Store};

pub(super) type Test = Result<(), Box<dyn StdError>>;
pub(super) type Boxed<T> = Result<T, Box<dyn StdError>>;

/// A scratch dir holding `project/` and the data dir `data/`, removed on
/// drop.
pub(super) struct Scratch {
    dir: PathBuf,
    pub(super) cfg: Config,
}

impl Scratch {
    pub(super) fn new(name: &str) -> Boxed<Self> {
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("chert-mcp-{pid}-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("project"))?;
        let mut cfg = load_from(None, &dir.join("project"))?.config;
        cfg.storage.data_dir = Some(dir.join("data"));
        Ok(Self { dir, cfg })
    }

    pub(super) fn root(&self) -> PathBuf {
        self.dir.join("project")
    }

    /// Append rows of session `s1`, 1 per body a second apart, from
    /// [`BASE`]; their refs in order.
    pub(super) fn append(&self, bodies: &[&str]) -> Boxed<Vec<Ref>> {
        let mut store = Store::open(&self.cfg, &self.root(), Mode::Read)?;
        let mut refs = Vec::new();
        for (ts, body) in (BASE..).step_by(1000).zip(bodies) {
            match store.append(&event(ts, body))? {
                Appended::Stored(r) => refs.push(r),
                Appended::Spooled => return Err("spooled".into()),
            }
        }
        Ok(refs)
    }
}

fn event(ts: u64, body: &str) -> Event {
    Event {
        ts,
        session: "s1".into(),
        kind: "post-tool-use".into(),
        ide: "claude-code".into(),
        identity: None,
        body: body.into(),
    }
}

/// 2026-01-01T00:00:00Z.
pub(super) const BASE: u64 = 1_767_225_600_000;

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Every line the server logged.
pub(super) type Lines = Arc<Mutex<Vec<String>>>;

/// The system clock and memory, the log kept in `lines`.
pub(super) fn env(lines: &Lines) -> Env {
    let kept = Arc::clone(lines);
    let log = Arc::new(move |text: &str| {
        if let Ok(mut all) = kept.lock() {
            all.push(text.to_owned());
        }
    });
    Env {
        log,
        ..Env::system()
    }
}

/// Run 1 session of stdin `input` and give back every stdout line; each
/// must be a JSON-RPC 2.0 frame (V2).
pub(super) fn raw(s: &Scratch, env: Env, input: &str) -> Boxed<Vec<Value>> {
    let root = s.root();
    let ctx = Ctx {
        config: &s.cfg,
        root: &root,
        env,
    };
    let mut out = Vec::new();
    serve(&ctx, input.as_bytes(), &mut out)?;
    frames(out)
}

/// Every stdout line as a JSON-RPC 2.0 frame (V2).
fn frames(out: Vec<u8>) -> Boxed<Vec<Value>> {
    let replies: Vec<Value> = String::from_utf8(out)?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    for r in &replies {
        assert_eq!(r.get("jsonrpc"), Some(&json!("2.0")), "{r}");
    }
    Ok(replies)
}

/// [`raw`] with `frames`, 1 per line.
pub(super) fn session_with(
    s: &Scratch,
    env: Env,
    frames: &[Value],
) -> Boxed<Vec<Value>> {
    let lines: Vec<String> = frames.iter().map(Value::to_string).collect();
    raw(s, env, &format!("{}\n", lines.join("\n")))
}

pub(super) fn session(s: &Scratch, frames: &[Value]) -> Boxed<Vec<Value>> {
    session_with(s, env(&Lines::default()), frames)
}

pub(super) fn req(id: u64, method: &str, params: &Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

#[test]
fn initialize_then_ping_answers_each_request_once() -> Test {
    let s = Scratch::new("init")?;
    let init = req(1, "initialize", &json!({"protocolVersion": "2025-06-18"}));
    let note = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
    let got = session(&s, &[init, note, req(2, "ping", &Value::Null)])?;
    assert_eq!(got.len(), 2);
    let result = got.first().and_then(|r| r.get("result"));
    let version = result.and_then(|r| r.get("protocolVersion"));
    assert_eq!(version, Some(&json!("2025-06-18")));
    let pong = json!({"jsonrpc": "2.0", "id": 2, "result": {}});
    assert_eq!(got.get(1), Some(&pong));
    Ok(())
}

#[test]
fn an_unknown_method_is_method_not_found() -> Test {
    let s = Scratch::new("unknown")?;
    let got = session(&s, &[req(3, "resources/list", &Value::Null)])?;
    let code = got.first().and_then(|r| r.pointer("/error/code"));
    assert_eq!(code, Some(&json!(-32601)));
    Ok(())
}

#[test]
fn bad_lines_are_answered_and_the_session_goes_on() -> Test {
    let s = Scratch::new("bad")?;
    let ping = req(9, "ping", &Value::Null);
    let lines =
        raw(&s, env(&Lines::default()), &format!("\n  \n{{oops\n{ping}"))?;
    let code = lines.first().and_then(|r| r.pointer("/error/code"));
    assert_eq!(code, Some(&json!(-32700)));
    assert_eq!(lines.get(1).and_then(|r| r.get("id")), Some(&json!(9)));
    assert_eq!(lines.len(), 2);
    Ok(())
}

/// A stdout that is closed.
struct Closed;

impl Write for Closed {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("closed"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_closed_stdout_ends_the_session_with_an_io_error() -> Test {
    let s = Scratch::new("closed")?;
    let root = s.root();
    let ctx = Ctx {
        config: &s.cfg,
        root: &root,
        env: env(&Lines::default()),
    };
    let ping = format!("{}\n", req(1, "ping", &Value::Null));
    let got = serve(&ctx, ping.as_bytes(), Closed);
    assert!(matches!(got, Err(Error::Io { .. })), "{got:?}");
    Ok(())
}

/// The log once a line holding `needle` arrives, waiting up to 10 s
/// for the cache thread.
fn logged(lines: &Lines, needle: &str) -> Boxed<Vec<String>> {
    let start = Instant::now();
    loop {
        let all = lines.lock().map_err(|_| "poisoned")?.clone();
        let late = start.elapsed() > Duration::from_secs(10);
        if late || all.iter().any(|l| l.contains(needle)) {
            return Ok(all);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn launch_starts_the_cache_and_its_warnings_reach_only_the_log() -> Test {
    let s = Scratch::new("cache")?;
    s.append(&["fix the auth bug"])?;
    let lines = Lines::default();
    let env = Env {
        // 1 KiB: any `cache.max_bytes` is over 25% of it.
        memory: || Some(1024),
        ..env(&lines)
    };
    let params = json!({"name": "search", "arguments": {"query": "auth"}});
    let got = session_with(&s, env, &[req(1, "tools/call", &params)])?;
    let all = logged(&lines, "cache.max_bytes")?;
    assert!(all.iter().any(|l| l.contains("over 25%")), "{all:?}");
    let text = got.first().map(Value::to_string).unwrap_or_default();
    assert!(!text.contains("isError") && text.contains("s1"), "{text}");
    assert!(!text.contains("cache.max_bytes"), "{text}");
    Ok(())
}
