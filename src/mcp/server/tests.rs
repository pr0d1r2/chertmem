//! Each tool over a whole session against a scratch store: names and
//! fields as cavemem 0.2.1 (V1), errors as tool results.

use std::fs;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::{ENGINE, GET, Server, WINDOW};
use crate::config::{Span, Unit};
use crate::mcp::Ctx;
use crate::mcp::tests::{
    BASE, Boxed, Lines, Scratch, Test, env, event, req, session,
};
use crate::store::{Appended, Mode, Ref, SessionStart, Store};

/// The result of calling tool `name` with `arguments` in a session.
fn tool(s: &Scratch, name: &str, arguments: &Value) -> Boxed<Value> {
    let params = json!({"name": name, "arguments": arguments});
    let got = session(s, &[req(1, "tools/call", &params)])?;
    let result = got.first().and_then(|r| r.get("result")).cloned();
    result.ok_or_else(|| format!("no result: {got:?}").into())
}

/// The JSON a tool result carries as its 1 text item.
fn payload(result: &Value) -> Boxed<Value> {
    let text = result.pointer("/content/0/text").and_then(Value::as_str);
    Ok(serde_json::from_str(text.ok_or("no text")?)?)
}

/// The keys of every object in the array `v`.
fn keys(v: &Value) -> Vec<Vec<String>> {
    let rows = v.as_array().cloned().unwrap_or_default();
    let keys = |r: &Value| r.as_object().map(|o| o.keys().cloned().collect());
    rows.iter().map(|r| keys(r).unwrap_or_default()).collect()
}

#[test]
fn tools_list_names_cavemems_4_tools() -> Test {
    let s = Scratch::new("list")?;
    let got = session(&s, &[req(1, "tools/list", &Value::Null)])?;
    let tools = got.first().and_then(|r| r.pointer("/result/tools"));
    let names: Vec<&str> = tools
        .and_then(Value::as_array)
        .map(|t| t.iter().filter_map(|t| t.get("name")?.as_str()).collect())
        .unwrap_or_default();
    let want = ["search", "timeline", "get_observations", "list_sessions"];
    assert_eq!(names, want);
    Ok(())
}

#[test]
fn search_answers_refs_with_cavemem_fields() -> Test {
    let s = Scratch::new("search")?;
    let refs = s.append(&["fix the auth bug", "unrelated note"])?;
    let got = payload(&tool(&s, "search", &json!({"query": "auth"}))?)?;
    let want = ["id", "score", "session_id", "snippet", "ts"];
    assert_eq!(keys(&got), [want.map(String::from).to_vec()]);
    let first = refs.first().map(ToString::to_string);
    assert_eq!(got.pointer("/0/id"), first.map(Value::from).as_ref());
    assert_eq!(got.pointer("/0/session_id"), Some(&json!("s1")));
    assert_eq!(got.pointer("/0/ts"), Some(&json!(BASE)));
    Ok(())
}

#[test]
fn timeline_answers_the_session_in_order() -> Test {
    let s = Scratch::new("timeline")?;
    let refs = s.append(&["one", "two", "three"])?;
    let args = json!({"session_id": "s1", "limit": 2});
    let got = payload(&tool(&s, "timeline", &args)?)?;
    let ids = refs.iter().take(2).map(ToString::to_string);
    let want: Vec<Value> = (BASE..)
        .step_by(1000)
        .zip(ids)
        .map(|(ts, id)| json!({"id": id, "kind": "post-tool-use", "ts": ts}))
        .collect();
    assert_eq!(got, Value::from(want));
    Ok(())
}

#[test]
fn get_observations_compressed_unless_expand_and_dropped_kept() -> Test {
    let s = Scratch::new("get")?;
    let body = "fix auth mw";
    let refs = s.append(&[body])?;
    let id = refs.first().map(ToString::to_string).ok_or("no ref")?;
    let ids = json!([id, "chert:01KZZZZZ"]);
    let got = payload(&tool(&s, "get_observations", &json!({"ids": ids}))?)?;
    assert_eq!(got.pointer("/0/content"), Some(&json!(body)));
    assert_eq!(got.pointer("/0/metadata"), Some(&Value::Null));
    let gone = json!({"id": "chert:01KZZZZZ", "dropped": true});
    assert_eq!(got.get(1), Some(&gone));
    let args = json!({"ids": [id], "expand": true});
    let got = payload(&tool(&s, "get_observations", &args)?)?;
    let expanded = crate::grammar::expand(body);
    assert_eq!(got.pointer("/0/content"), Some(&json!(expanded)));
    Ok(())
}

#[test]
fn a_bad_call_is_a_tool_error_not_a_protocol_error() -> Test {
    let s = Scratch::new("toolerr")?;
    let gone = json!({"session_id": "s1", "around_id": "chert:01KZZZZZ"});
    for (name, args) in [
        ("get_observations", json!({"ids": ["nope"]})),
        ("list_sessions", json!({"limit": 201})),
        ("timeline", gone),
    ] {
        let got = tool(&s, name, &args)?;
        assert_eq!(got.get("isError"), Some(&json!(true)), "{name}: {got}");
    }
    Ok(())
}

/// When `s2` starts, a second after `s1` at [`BASE`].
const LATER: u64 = BASE + 1000;
/// When `s1` ends.
const ENDED: u64 = BASE + 500;

/// Start sessions `s1` at [`BASE`] and `s2` at [`LATER`]; end `s1`.
fn start_two(s: &Scratch) -> Test {
    let mut store = Store::open(&s.cfg, &s.root(), Mode::Read)?;
    for (id, ts) in [("s1", BASE), ("s2", LATER)] {
        let start = SessionStart {
            id: id.into(),
            ide: "claude-code".into(),
            cwd: "/work/p".into(),
            ts,
        };
        store.session_started(&start)?;
    }
    store.session_ended("s1", ENDED)?;
    Ok(())
}

fn row(id: &str, started_at: u64, ended_at: Option<u64>) -> Value {
    json!({"id": id, "ide": "claude-code", "cwd": "/work/p",
        "started_at": started_at, "ended_at": ended_at})
}

#[test]
fn list_sessions_answers_newest_first_ended_at_null_while_running() -> Test {
    let s = Scratch::new("sessions")?;
    start_two(&s)?;
    let got = payload(&tool(&s, "list_sessions", &json!({}))?)?;
    let want = json!([row("s2", LATER, None), row("s1", BASE, Some(ENDED))]);
    assert_eq!(got, want);
    let one = payload(&tool(&s, "list_sessions", &json!({"limit": 1}))?)?;
    assert_eq!(one, json!([row("s2", LATER, None)]));
    Ok(())
}

// T5: the 3 parts of a server share 1 budget of partition connections.
// `ulimit` cannot be set without `unsafe`, so the test counts this
// process's open descriptors, as src/search V6's does.

/// The descriptors this process holds open, as its fd dir lists them.
fn open_fds() -> Boxed<usize> {
    let dir = if cfg!(target_os = "linux") {
        "/proc/self/fd"
    } else {
        "/dev/fd"
    };
    Ok(fs::read_dir(dir)?.count())
}

/// 3 descriptors a partition connection for at most 64 connections, and
/// slack for the catalogs and the terms file: under 256 with the rest.
const FD_BOUND: usize = 3 * 64 + 48;

/// 8 days, past the 7-day bucket: every row its own partition.
const STEP: u64 = 8 * 24 * 3600 * 1000;

/// 1 row in each of 520 partitions, newest first back from [`BASE`].
fn spread(s: &Scratch) -> Boxed<Vec<Ref>> {
    let mut store = Store::open(&s.cfg, &s.root(), Mode::Read)?;
    let mut refs = Vec::new();
    for i in 0..520_u64 {
        let ts = BASE.saturating_sub(i.saturating_mul(STEP));
        match store.append(&event(ts, "cargo test"))? {
            Appended::Stored(r) => refs.push(r),
            Appended::Spooled => return Err("spooled".into()),
        }
    }
    Ok(refs)
}

/// Wait up to 60 s for the cache thread to finish its warm-up.
fn warmed(server: &Server) -> Test {
    let start = Instant::now();
    while !server.cache.is_warm()? {
        if start.elapsed() > Duration::from_mins(1) {
            return Err("cache never warmed".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

/// Call tool `name` on `server`; it must not fail.
fn call(server: &mut Server, name: &str, arguments: &Value) -> Test {
    let params = json!({"name": name, "arguments": arguments});
    let line = req(1, "tools/call", &params).to_string();
    let got = server.reply(&line).ok_or("no reply")?;
    assert!(got.pointer("/result/isError").is_none(), "{got}");
    Ok(())
}

/// Reach every partition: warm-up the window's, `get_observations`
/// each row's at the tool's 50 refs a call, a search's pool the rest.
fn read_everything(server: &mut Server, refs: &[Ref]) -> Test {
    warmed(server)?;
    for chunk in refs.chunks(50) {
        let ids: Vec<String> = chunk.iter().map(ToString::to_string).collect();
        call(server, "get_observations", &json!({"ids": ids}))?;
    }
    call(server, "search", &json!({"query": "cargo"}))
}

/// A scratch of 520 partitions, half in the window and the rest for a
/// search's pool, and the refs of its rows.
fn spread_scratch() -> Boxed<(Scratch, Vec<Ref>)> {
    let mut s = Scratch::new("fds")?;
    s.cfg.cache.window = Span {
        amount: 2000,
        unit: Unit::Days,
    };
    let refs = spread(&s)?;
    Ok((s, refs))
}

#[test]
fn the_3_parts_share_1_store_budget() {
    let total = WINDOW.saturating_add(GET).saturating_add(ENGINE);
    assert_eq!(total, Store::KEEP_OPEN);
}

#[test]
fn a_server_over_520_partitions_holds_a_bounded_number_of_files() -> Test {
    let (s, refs) = spread_scratch()?;
    let (before, root) = (open_fds()?, s.root());
    let lines = Lines::default();
    let (config, env) = (&s.cfg, env(&lines));
    let mut server = Server::open(&Ctx {
        config,
        root: &root,
        env,
    })?;
    read_everything(&mut server, &refs)?;
    let held = open_fds()?.saturating_sub(before);
    eprintln!("mcp T5: {before} fds before, {held} more with a server up");
    assert!(held <= FD_BOUND, "{held} fds");
    Ok(())
}
