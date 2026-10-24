use super::{findings, id};
use crate::mine::tests::{Res, Test, bash, reference, row};
use crate::mine::{Finding, Kind};
use crate::store::{Observation, Ref};

/// Bash rows: `(n, session, command)` each.
fn calls(spec: &[(u64, &str, &str)]) -> Res<Vec<Observation>> {
    spec.iter().map(|&(n, s, c)| bash(n, s, c)).collect()
}

fn refs(ns: &[u64]) -> Res<Vec<Ref>> {
    ns.iter().map(|&n| reference(n)).collect()
}

fn of(found: &[Finding], kind: Kind) -> Vec<&Finding> {
    found.iter().filter(|f| f.kind == kind).collect()
}

#[test]
fn a_pair_repeated_in_1_session_is_a_sequence() -> Test {
    let (fmt, test) = ("cargo fmt", "cargo test");
    let spec = [(1, "s", fmt), (2, "s", test), (3, "s", fmt)];
    let more = [(4, "s", test), (5, "s", fmt), (6, "s", test)];
    let found = findings(&calls(&[spec, more].concat())?, 3);
    let seqs = of(&found, Kind::Sequence);
    let [only] = seqs.as_slice() else {
        return Err(format!("not 1 sequence: {found:?}").into());
    };
    assert_eq!(only.count, 3);
    assert_eq!(only.refs, refs(&[1, 3, 5])?);
    assert_eq!(only.summary, "`cargo fmt` then `cargo test`");
    Ok(())
}

#[test]
fn a_sequence_never_crosses_sessions() -> Test {
    let (make, check) = ("make", "make check");
    let spec = [(1, "a", make), (2, "b", make), (3, "a", check)];
    let more = [(4, "c", make), (5, "b", check), (6, "c", check)];
    let found = findings(&calls(&[spec, more].concat())?, 3);
    let seqs = of(&found, Kind::Sequence);
    let counts: Vec<_> = seqs.iter().map(|f| (f.count, &f.summary)).collect();
    let summary = "`make` then `make check`".to_owned();
    assert_eq!(counts, [(3, &summary)]);
    let first = seqs.first().map(|f| f.refs.clone()).unwrap_or_default();
    assert_eq!(first, refs(&[1, 2, 4])?);
    Ok(())
}

#[test]
fn fewer_than_min_occurrences_is_no_finding() -> Test {
    let rows = calls(&[(1, "s", "ls"), (2, "s", "pwd"), (3, "s", "ls")])?;
    let found = findings(&rows, 2);
    assert!(of(&found, Kind::Sequence).is_empty(), "{found:?}");
    Ok(())
}

#[test]
fn other_tools_and_kinds_are_skipped() -> Test {
    let mut rows = calls(&[(1, "s", "ls"), (3, "s", "pwd")])?;
    rows.push(row(2, "s", "Read", r#"{"file_path":"x"}"#)?);
    let mut prompt = bash(4, "s", "ls")?;
    prompt.kind = "user-prompt-submit".to_owned();
    rows.push(prompt);
    let found = findings(&rows, 1);
    let seqs: Vec<_> = of(&found, Kind::Sequence)
        .iter()
        .map(|f| f.summary.clone())
        .collect();
    assert_eq!(seqs, ["`ls` then `pwd`"]);
    Ok(())
}

#[test]
fn the_same_rows_in_any_order_give_the_same_findings() -> Test {
    let spec = [(1, "s", "a"), (2, "s", "b"), (3, "s", "a"), (4, "s", "b")];
    let rows = calls(&spec)?;
    let mut shuffled = rows.clone();
    shuffled.reverse();
    let found = findings(&rows, 1);
    assert_eq!(found, findings(&shuffled, 1));
    let counts: Vec<_> = found.iter().map(|f| f.count).collect();
    assert_eq!(counts, [2, 1]);
    Ok(())
}

#[test]
fn a_summary_stays_on_1_short_line() -> Test {
    let long = format!("echo 'x\ny' {}", "z".repeat(200));
    let rows = calls(&[(1, "s", &long), (2, "s", "ls")])?;
    let found = findings(&rows, 1);
    let summary = found.first().map(|f| f.summary.clone());
    let summary = summary.unwrap_or_default();
    assert!(!summary.contains('\n'), "{summary}");
    assert!(summary.chars().count() < 200, "{summary}");
    assert!(summary.starts_with("`echo 'x y' zzz"), "{summary}");
    Ok(())
}

#[test]
fn an_id_is_the_fnv1a_64_of_its_pattern() {
    assert_eq!(id(""), "cbf29ce484222325");
    assert_eq!(id("a"), "af63dc4c8601ec8c");
}

#[test]
fn the_same_pattern_has_the_same_id_and_others_differ() -> Test {
    let rows = calls(&[(1, "s", "a"), (2, "s", "b"), (3, "s", "a")])?;
    let found = findings(&rows, 1);
    let ids: Vec<_> = found.iter().map(|f| f.id.clone()).collect();
    let again: Vec<_> = findings(&rows, 1).into_iter().map(|f| f.id).collect();
    assert_eq!(ids, again);
    assert_eq!(ids.len(), 2);
    assert!(ids.first() != ids.get(1));
    Ok(())
}

#[test]
fn a_path_named_by_enough_calls_is_a_hot_file() -> Test {
    let read = |n, s| row(n, s, "Read", r#"{"file_path":"src/lib.rs"}"#);
    let edit = r#"{"file_path":"src/lib.rs","path":"src/lib.rs"}"#;
    let mut rows = vec![read(4, "b")?, read(1, "a")?];
    rows.push(row(2, "a", "Edit", edit)?);
    rows.extend(calls(&[(3, "a", "cat src/lib.rs")])?);
    let found = findings(&rows, 3);
    let hot = of(&found, Kind::HotFile);
    let [only] = hot.as_slice() else {
        return Err(format!("not 1 hot file: {found:?}").into());
    };
    assert_eq!((only.count, only.refs.clone()), (3, refs(&[1, 2, 4])?));
    assert_eq!(only.summary, "hot file `src/lib.rs`");
    Ok(())
}

#[test]
fn a_path_named_too_few_times_is_no_hot_file() -> Test {
    let read = |n| row(n, "s", "Read", r#"{"file_path":"src/main.rs"}"#);
    let found = findings(&[read(1)?, read(2)?], 3);
    assert!(of(&found, Kind::HotFile).is_empty(), "{found:?}");
    Ok(())
}

#[test]
fn hot_files_and_sequences_share_1_order() -> Test {
    let read = |n| row(n, "s", "Read", r#"{"file_path":"README.md"}"#);
    let mut rows = calls(&[(1, "s", "ls"), (2, "s", "pwd"), (3, "s", "ls")])?;
    rows.extend([read(4)?, read(5)?, read(6)?]);
    let found = findings(&rows, 1);
    let kinds: Vec<_> = found.iter().map(|f| (f.kind, f.count)).collect();
    let seq = Kind::Sequence;
    assert_eq!(kinds, [(Kind::HotFile, 3), (seq, 1), (seq, 1)]);
    Ok(())
}
