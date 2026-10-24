use std::io::Write;

use serde_json::Value;

use super::super::Error;
use super::super::tests::{NOW, Res, Scratch, Test, event};
use super::{export, line};
use crate::hook::{self, Ide, Kind};
use crate::store::{self, Filter, Mode, Observation, Store};

/// The export of `store` with `filter`, and its line count.
fn exported(store: &mut Store, filter: &Filter) -> Res<(String, usize)> {
    let mut out = Vec::new();
    let n = export(store, filter, &mut out)?;
    Ok((String::from_utf8(out)?, n))
}

/// Field `key` of `line` as text; `None` for null.
fn text(line: &Value, key: &str) -> Option<String> {
    line.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// Whether `line` holds every field of `o`, its ref as `chert:<ref>`.
fn same(line: &Value, o: &Observation) -> bool {
    let ts = line.get("ts").and_then(Value::as_u64);
    text(line, "ref") == Some(o.reference.to_string())
        && text(line, "id") == Some(o.id.clone())
        && text(line, "project") == Some(o.project.clone())
        && ts == Some(o.ts)
        && text(line, "session") == Some(o.session.clone())
        && text(line, "ide") == Some(o.ide.clone())
        && text(line, "kind") == Some(o.kind.clone())
        && text(line, "identity") == o.identity
        && text(line, "body") == Some(o.body.clone())
}

#[test]
fn every_line_is_1_observation_with_its_ref() -> Test {
    let s = Scratch::new("lines")?;
    let mut store = s.store()?;
    store.append(&event(2, "b", Some("tool-1")))?;
    store.append(&event(1, "a \"quoted\"\nline", None))?;
    store.append(&event(3, "c", None))?;
    let rows = store.observations(&Filter::default())?;
    let (out, n) = exported(&mut store, &Filter::default())?;
    assert!(n == 3 && out.ends_with('\n'));
    let lines = parsed(&out)?;
    assert_eq!(lines.len(), rows.len());
    assert!(lines.iter().zip(&rows).all(|(l, o)| same(l, o)));
    Ok(())
}

/// Every line of `out` as JSON.
fn parsed(out: &str) -> Res<Vec<Value>> {
    let lines = out.lines().map(serde_json::from_str);
    Ok(lines.collect::<Result<_, _>>()?)
}

#[test]
fn a_line_has_null_identity_and_sorted_keys() -> Test {
    let s = Scratch::new("shape")?;
    let mut store = s.store()?;
    store.append(&event(0, "x", None))?;
    let rows = store.observations(&Filter::default())?;
    let one = line(rows.first().ok_or("no row")?);
    assert!(one.starts_with("{\"body\":\"x\""), "{one}");
    assert!(one.contains("\"identity\":null"), "{one}");
    assert!(!one.contains('\n'));
    Ok(())
}

#[test]
fn the_filter_picks_the_rows() -> Test {
    let s = Scratch::new("filter")?;
    let mut store = s.store()?;
    store.append(&event(0, "old", None))?;
    store.append(&event(10, "new", None))?;
    let since = Filter {
        since: Some(NOW.saturating_add(5)),
        ..Filter::default()
    };
    let (out, n) = exported(&mut store, &since)?;
    assert_eq!(n, 1);
    assert!(out.contains("\"new\"") && !out.contains("\"old\""));
    Ok(())
}

#[test]
fn an_empty_store_exports_nothing() -> Test {
    let s = Scratch::new("empty")?;
    let (out, n) = exported(&mut s.store()?, &Filter::default())?;
    assert_eq!((out.as_str(), n), ("", 0));
    Ok(())
}

/// A prompt holding a private span, as Claude Code sends it.
const PROMPT: &str = r#"{"session_id":"abc123","cwd":"/w",
"hook_event_name":"UserPromptSubmit",
"prompt":"deploy <private>hunter2-secret</private> now"}"#;

/// Run the prompt hook on [`PROMPT`] in `s`, as Claude Code would.
fn prompted(s: &Scratch) -> u8 {
    let root = s.root();
    let ctx = hook::Ctx {
        config: &s.config,
        root: &root,
        now: NOW,
        ide: Ide::ClaudeCode,
    };
    hook::run(Kind::UserPromptSubmit, PROMPT.as_bytes(), &ctx).exit
}

#[test]
fn a_private_span_never_reaches_the_export() -> Test {
    let s = Scratch::new("private")?;
    assert_eq!(prompted(&s), 0);
    let mut store = s.store()?;
    let rows = store.observations(&Filter::default())?;
    let (out, n) = exported(&mut store, &Filter::default())?;
    assert_eq!(n, 1);
    assert!(!out.contains("hunter2"), "{out}");
    let first: Value = serde_json::from_str(out.trim_end())?;
    let stored = rows.first().ok_or("no row")?;
    assert_eq!(text(&first, "body"), Some(stored.body.clone()));
    Ok(())
}

/// A sink that refuses every write.
struct Broken;

impl Write for Broken {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("refused"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::other("refused"))
    }
}

#[test]
fn a_failed_write_is_an_io_error() -> Test {
    let s = Scratch::new("broken")?;
    let mut store = s.store()?;
    store.append(&event(0, "x", None))?;
    let got = export(&mut store, &Filter::default(), &mut Broken);
    let reason = "refused".to_owned();
    assert_eq!(got, Err(Error::Io { reason }));
    let other = Scratch::new("broken-empty")?;
    let mut empty = other.store()?;
    let got = export(&mut empty, &Filter::default(), &mut Broken);
    assert!(matches!(got, Err(Error::Io { .. })));
    Ok(())
}

#[test]
fn a_write_only_store_cannot_export() -> Test {
    let s = Scratch::new("write-only")?;
    let mut store = s.open(Mode::Write)?;
    let got = export(&mut store, &Filter::default(), &mut Vec::new());
    assert_eq!(got, Err(Error::Store(store::Error::WriteOnly)));
    Ok(())
}
