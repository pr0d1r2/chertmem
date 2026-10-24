use std::io::{BufReader, Read};

use serde_json::Value;

use super::super::tests::{Res, Scratch, Test, event};
use super::super::{Error, export};
use super::jsonl;
use crate::store::{self, Filter, Lookup, Mode, Observation, Store};

/// An event `at` ms after the test epoch in `session`.
fn said(at: u64, session: &str, body: &str) -> store::Event {
    let mut e = event(at, body, None);
    session.clone_into(&mut e.session);
    e
}

/// A store holding 2 projects, each with 2 sessions; seconds apart, so
/// no 2 projects issue the same ref.
fn filled(s: &Scratch) -> Res<Store> {
    let mut other = Store::open(&s.config, &s.path("other"), Mode::Read)?;
    other.append(&said(5_000, "o1", "elsewhere"))?;
    other.append(&said(5_001, "o2", "elsewhere too"))?;
    drop(other);
    let mut store = s.store()?;
    store.append(&said(0, "s1", "first"))?;
    store.append(&said(1, "s2", "second"))?;
    store.append(&event(2, "tool", Some("tool-1")))?;
    Ok(store)
}

/// `store`'s export and its rows.
fn exported(store: &mut Store) -> Res<(Vec<u8>, Vec<Observation>)> {
    let mut out = Vec::new();
    export(store, &Filter::default(), &mut out)?;
    Ok((out, store.observations(&Filter::default())?))
}

/// Whether every ref of `rows` resolves in `store` to its own row.
fn resolves(store: &mut Store, rows: &[Observation]) -> Res<bool> {
    for row in rows {
        let got = store.get(std::slice::from_ref(&row.reference))?;
        if got != [Lookup::Found(row.clone())] {
            return Ok(false);
        }
    }
    Ok(true)
}

#[test]
fn an_export_imports_into_an_empty_store_field_for_field() -> Test {
    let (from, to) = (Scratch::new("jl-from")?, Scratch::new("jl-to")?);
    let (out, rows) = exported(&mut filled(&from)?)?;
    assert_eq!(rows.len(), 5);
    let mut store = to.store()?;
    assert_eq!(jsonl(&mut store, out.as_slice())?, 5);
    assert_eq!(store.observations(&Filter::default())?, rows);
    assert!(resolves(&mut store, &rows)?);
    Ok(())
}

#[test]
fn importing_twice_adds_nothing() -> Test {
    let (from, to) = (Scratch::new("jl-src")?, Scratch::new("jl-twice")?);
    let (out, rows) = exported(&mut filled(&from)?)?;
    let mut store = to.store()?;
    jsonl(&mut store, out.as_slice())?;
    assert_eq!(jsonl(&mut store, out.as_slice())?, 5);
    assert_eq!(store.observations(&Filter::default())?, rows);
    Ok(())
}

/// `line` with its `id` replaced by another id of the same ms.
fn reissued(line: &str, id: &str) -> Res<String> {
    let last = if id.ends_with('Z') { "Y" } else { "Z" };
    let other = format!("{}{last}", id.get(..25).ok_or("short id")?);
    let mut value: Value = serde_json::from_str(line)?;
    let map = value.as_object_mut().ok_or("not an object")?;
    map.insert("id".to_owned(), Value::from(other));
    Ok(value.to_string())
}

/// An export of 1 row, a blank line, then that row under another id.
fn twice_issued() -> Res<String> {
    let from = Scratch::new("jl-a")?;
    let mut src = from.store()?;
    src.append(&said(0, "s1", "only"))?;
    let (out, rows) = exported(&mut src)?;
    let first = String::from_utf8(out)?;
    let row = rows.first().ok_or("no row")?;
    let again = reissued(first.trim_end(), &row.id)?;
    Ok(format!("{first}\n{again}\n"))
}

#[test]
fn a_ref_another_id_holds_names_its_line() -> Test {
    let (line, cause) = failed("jl-taken", &twice_issued()?)?;
    assert_eq!(line, 3);
    let taken = matches!(cause, Error::Store(store::Error::RefTaken { .. }));
    assert!(taken, "{cause}");
    Ok(())
}

/// The line and cause importing `input` into an empty store fails with.
fn failed(name: &str, input: &str) -> Res<(usize, Error)> {
    let s = Scratch::new(name)?;
    match jsonl(&mut s.store()?, input.as_bytes()) {
        Err(Error::Line { line, cause }) => Ok((line, *cause)),
        other => Err(format!("{other:?}").into()),
    }
}

#[test]
fn a_line_that_is_not_an_export_line_is_refused() -> Test {
    let (line, cause) = failed("jl-json", "\nnot json\n")?;
    assert_eq!(line, 2);
    assert!(matches!(cause, Error::Jsonl { .. }), "{cause}");
    let (_, cause) = failed("jl-field", "{\"ref\":\"chert:0123456789\"}")?;
    assert!(cause.to_string().contains("`id`"), "{cause}");
    let (_, cause) = failed("jl-ref", "{\"ref\":\"x\"}")?;
    assert!(matches!(cause, Error::Store(store::Error::BadRef { .. })));
    Ok(())
}

#[test]
fn a_wrong_identity_or_ts_is_refused() -> Test {
    let line = "{\"body\":\"b\",\"id\":\"01K0000000000000000000000A\",\
      \"ide\":\"i\",\"identity\":1,\"kind\":\"k\",\"project\":\"p\",\
      \"ref\":\"chert:01K00000\",\"session\":\"s\",\"ts\":1}";
    let (_, cause) = failed("jl-identity", line)?;
    assert!(cause.to_string().contains("`identity`"), "{cause}");
    let fixed = line.replace("\"identity\":1", "\"identity\":null");
    let no_ts = fixed.replace("\"ts\":1", "\"ts\":-1");
    let (_, cause) = failed("jl-ts", &no_ts)?;
    assert!(cause.to_string().contains("`ts`"), "{cause}");
    Ok(())
}

/// A reader that refuses every read.
struct Broken;

impl Read for Broken {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("unplugged"))
    }
}

#[test]
fn an_unreadable_input_names_line_1() -> Test {
    let s = Scratch::new("jl-broken")?;
    let got = jsonl(&mut s.store()?, BufReader::new(Broken));
    let Err(Error::Line { line: 1, cause }) = got else {
        return Err(format!("{got:?}").into());
    };
    assert!(cause.to_string().contains("unplugged"), "{cause}");
    Ok(())
}
