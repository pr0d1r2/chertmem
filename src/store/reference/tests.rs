use std::error::Error as StdError;
use std::fs;

use super::{Ref, candidates};
use crate::store::tests::{Scratch, Test, chunk, event, stored};
use crate::store::{Error, Lookup, Mode, Store};

const ID: &str = "01JB2X3Y4Z5A6B7C8D9E0F1G2H";

#[test]
fn a_ref_shows_as_chert_colon() -> Test {
    let r = Ref::parse("chert:01JB2X3Y")?;
    assert_eq!(r.to_string(), "chert:01JB2X3Y");
    assert_eq!(r.as_str(), "01JB2X3Y");
    assert_eq!(Ref::parse(&format!("chert:{ID}"))?.as_str(), ID);
    Ok(())
}

#[test]
fn parse_takes_only_the_issued_form() {
    let bad = [
        "01JB2X3Y",
        "chert:",
        "chert:01JB2X3",
        "chert:01jb2x3y",
        "chert:01JB2X3I",
        "chert:01JB2X3Y4Z5A6B7C8D9E0F1G2HX",
        "ref:01JB2X3Y",
    ];
    for text in bad {
        let want = Error::BadRef { text: text.into() };
        assert_eq!(Ref::parse(text), Err(want), "{text}");
    }
}

#[test]
fn candidates_run_from_8_chars_to_the_whole_id() {
    let all: Vec<&str> = candidates(ID).collect();
    assert_eq!(all.len(), 19);
    assert_eq!((all.first(), all.last()), (Some(&"01JB2X3Y"), Some(&ID)));
}

fn found(store: &mut Store, r: &Ref) -> Result<String, Box<dyn StdError>> {
    match store.get(std::slice::from_ref(r))?.pop() {
        Some(Lookup::Found(o)) if &o.reference == r => Ok(o.body),
        other => Err(format!("{r}: {other:?}").into()),
    }
}

#[test]
fn a_lone_row_gets_8_chars_and_resolves() -> Test {
    let s = Scratch::new("ref-short")?;
    let mut store = s.open(Mode::Read)?;
    let r = stored(store.append(&event(1_709_211_600_000, None, "a"))?)?;
    assert_eq!(r.as_str().len(), 8);
    assert_eq!(found(&mut store, &r)?, "a");
    assert_eq!(found(&mut store, &Ref::parse(&r.to_string())?)?, "a");
    Ok(())
}

#[test]
fn rows_in_one_millisecond_get_longer_refs() -> Test {
    let s = Scratch::new("ref-longer")?;
    let mut store = s.open(Mode::Read)?;
    let a = stored(store.append(&event(7, None, "a"))?)?;
    let b = stored(store.append(&event(7, None, "b"))?)?;
    let c = stored(store.append(&event(7, None, "c"))?)?;
    // Equal to no issued ref is all it takes: 9 chars differ from 8.
    let lens = [a.as_str().len(), b.as_str().len(), c.as_str().len()];
    assert_eq!(lens, [8, 9, 10]);
    assert!(c.as_str().starts_with(b.as_str()));
    assert!(b.as_str().starts_with(a.as_str()));
    let got = store.get(&[c, b, a])?;
    assert_eq!(bodies(&got), ["c", "b", "a"]);
    Ok(())
}

/// What 1 ref resolves to.
fn one(store: &mut Store, r: &Ref) -> Result<Lookup, Box<dyn StdError>> {
    let got = store.get(std::slice::from_ref(r))?;
    got.into_iter().next().ok_or_else(|| "no answer".into())
}

fn bodies(got: &[Lookup]) -> Vec<&str> {
    got.iter().map(body).collect()
}

fn body(l: &Lookup) -> &str {
    match l {
        Lookup::Found(o) => o.body.as_str(),
        Lookup::Dropped(_) => "dropped",
    }
}

#[test]
fn a_ref_never_issued_is_dropped() -> Test {
    let s = Scratch::new("ref-unknown")?;
    let mut store = s.open(Mode::Read)?;
    let a = stored(store.append(&event(7, None, "a"))?)?;
    // An issued ref is never a prefix to search under.
    let longer = Ref::parse(&format!("{a}0000"))?;
    let none = Ref::parse("chert:7ZZZZZZZ")?;
    let got = store.get(&[longer.clone(), none.clone()])?;
    assert_eq!(got, [Lookup::Dropped(longer), Lookup::Dropped(none)]);
    assert_eq!(found(&mut store, &a)?, "a");
    Ok(())
}

#[test]
fn a_gone_row_is_dropped_and_its_ref_never_reissued() -> Test {
    let s = Scratch::new("ref-gone-row")?;
    let mut store = s.open(Mode::Read)?;
    let a = stored(store.append(&event(7, None, "a"))?)?;
    let sql = "DELETE FROM observations";
    s.sql(&chunk(&store, "19700101-7d"))?.execute(sql, [])?;
    assert_eq!(one(&mut store, &a)?, Lookup::Dropped(a.clone()));
    let b = stored(store.append(&event(7, None, "b"))?)?;
    assert_ne!(a, b);
    assert_eq!(one(&mut store, &a)?, Lookup::Dropped(a.clone()));
    Ok(())
}

#[test]
fn a_gone_partition_is_dropped() -> Test {
    let s = Scratch::new("ref-gone-part")?;
    let mut store = s.open(Mode::Read)?;
    let a = stored(store.append(&event(7, None, "a"))?)?;
    let path = chunk(&store, "19700101-7d");
    drop(store);
    fs::remove_file(s.data().join(&path))?;
    let mut store = s.open(Mode::Read)?;
    assert_eq!(one(&mut store, &a)?, Lookup::Dropped(a.clone()));
    let sql = "DELETE FROM refs; DELETE FROM partitions;";
    s.sql("catalog.db")?.execute_batch(sql)?;
    assert_eq!(one(&mut store, &a)?, Lookup::Dropped(a.clone()));
    Ok(())
}

#[test]
fn a_write_only_store_resolves_nothing() -> Test {
    let s = Scratch::new("ref-write-only")?;
    let mut store = s.open(Mode::Write)?;
    let a = stored(store.append(&event(7, None, "a"))?)?;
    assert_eq!(store.get(&[a]).err(), Some(Error::WriteOnly));
    Ok(())
}
