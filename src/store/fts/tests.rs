use std::error::Error as StdError;

use crate::store::tests::{Scratch, Test, event, stored};
use crate::store::{Error, Filter, Frequencies, Mode, Store};

/// A store holding `bodies` at ts 10, 20, ..., all in 1 partition;
/// that partition's path.
fn filled(
    s: &Scratch,
    bodies: &[&str],
) -> Result<(Store, String), Box<dyn StdError>> {
    let mut store = s.open(Mode::Read)?;
    for (ts, body) in (10..).step_by(10).zip(bodies) {
        stored(store.append(&event(ts, None, body))?)?;
    }
    let all = store.partitions(&Filter::default())?;
    let path = all.first().map(|p| p.path.clone()).ok_or("no partition")?;
    Ok((store, path))
}

fn terms(words: &[&str]) -> Vec<String> {
    words.iter().map(|&w| w.to_owned()).collect()
}

/// The bodies `words` match in `path`, by seq.
fn hits(
    store: &mut Store,
    path: &str,
    words: &[&str],
    filter: &Filter,
) -> Result<Vec<String>, Box<dyn StdError>> {
    let rows = store.matches(path, &terms(words), filter)?;
    Ok(rows.into_iter().map(|t| t.observation.body).collect())
}

const BODIES: &[&str] = &[
    "cargo build failed",
    "tests pass",
    "nothing here",
    "Cargo test PASS",
];

#[test]
fn any_term_matches_and_rows_come_by_seq() -> Test {
    let s = Scratch::new("fts-any")?;
    let (mut store, path) = filled(&s, BODIES)?;
    let all = Filter::default();
    let found = hits(&mut store, &path, &["cargo", "pass"], &all)?;
    assert_eq!(
        found,
        ["cargo build failed", "tests pass", "Cargo test PASS"]
    );
    let rows = store.matches(&path, &terms(&["failed"]), &all)?;
    let seqs: Vec<i64> = rows.iter().map(|t| t.seq).collect();
    assert_eq!(seqs, [1]);
    assert_eq!(hits(&mut store, &path, &["absent"], &all)?, [""; 0]);
    Ok(())
}

#[test]
fn the_filter_bounds_project_and_time() -> Test {
    let s = Scratch::new("fts-filter")?;
    let (mut store, path) = filled(&s, BODIES)?;
    let window = Filter {
        project: Some(store.project().to_owned()),
        since: Some(15),
        until: Some(40),
    };
    let found = hits(&mut store, &path, &["cargo", "pass"], &window)?;
    assert_eq!(found, ["tests pass"]);
    let other = Filter {
        project: Some("other".into()),
        ..Filter::default()
    };
    assert_eq!(hits(&mut store, &path, &["cargo"], &other)?, [""; 0]);
    Ok(())
}

const SYNTAX: &[&str] = &[
    "say \"hi\" there",
    "NEAR(a b) at the edge",
    "col:value pair",
    "x-y axis",
    "star* gazing",
];

/// Terms full of FTS5 syntax and the bodies each must match as text.
const CASES: &[(&[&str], &[&str])] = &[
    (&["\"hi\""], &["say \"hi\" there"]),
    (&["NEAR(a b)"], &["NEAR(a b) at the edge"]),
    (&["NEAR"], &["NEAR(a b) at the edge"]),
    (&["col:value"], &["col:value pair"]),
    (&["-y"], &["x-y axis"]),
    (&["star*"], &["star* gazing"]),
    (&["sta*"], &[]),
    (&["*", "\"", ":", "-", "a\" OR \"edge"], &[]),
    (&["NOT", "AND", "OR", "say) OR (edge"], &[]),
];

#[test]
fn fts5_syntax_in_a_term_is_only_text() -> Test {
    let s = Scratch::new("fts-syntax")?;
    let (mut store, path) = filled(&s, SYNTAX)?;
    let all = Filter::default();
    for &(words, want) in CASES {
        assert_eq!(hits(&mut store, &path, words, &all)?, want, "{words:?}");
    }
    Ok(())
}

#[test]
fn no_terms_a_gone_file_or_a_write_only_store_hold_nothing() -> Test {
    let s = Scratch::new("fts-none")?;
    let (mut store, path) = filled(&s, BODIES)?;
    let all = Filter::default();
    assert_eq!(hits(&mut store, &path, &[], &all)?, [""; 0]);
    assert_eq!(hits(&mut store, &path, &["", "  "], &all)?, [""; 0]);
    assert_eq!(
        hits(&mut store, "chunks/x/none.db", &["cargo"], &all)?,
        [""; 0]
    );
    let mut hook = s.open(Mode::Write)?;
    let refused = hook.matches(&path, &terms(&["cargo"]), &all).err();
    assert_eq!(refused, Some(Error::WriteOnly));
    let refused = hook.frequencies(&path, &terms(&["cargo"])).err();
    assert_eq!(refused, Some(Error::WriteOnly));
    Ok(())
}

#[test]
fn frequencies_count_rows_and_rows_per_token() -> Test {
    let s = Scratch::new("fts-freq")?;
    let (mut store, path) = filled(&s, BODIES)?;
    let words = terms(&["cargo", "PASS", "here", "absent", "cargo build"]);
    let want = Frequencies {
        rows: 4,
        docs: vec![2, 2, 1, 0, 0],
    };
    assert_eq!(store.frequencies(&path, &words)?, want);
    let gone = store.frequencies("chunks/x/none.db", &words)?;
    let none = Frequencies {
        rows: 0,
        docs: vec![0; 5],
    };
    assert_eq!(gone, none);
    Ok(())
}
