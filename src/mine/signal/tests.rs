use super::{findings, id};
use crate::hook::inject::tokens_at_most;
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
    let seqs = of(&found, Kind::Sequence);
    let counts: Vec<_> = seqs.iter().map(|f| f.count).collect();
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
    let seq_ids = |rows: &[Observation]| -> Vec<String> {
        let found = findings(rows, 1);
        let seqs = of(&found, Kind::Sequence);
        seqs.iter().map(|f| f.id.clone()).collect()
    };
    let (ids, again) = (seq_ids(&rows), seq_ids(&rows));
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
    let shown = found.iter().filter(|f| f.kind != Kind::TokenSink);
    let kinds: Vec<_> = shown.map(|f| (f.kind, f.count)).collect();
    let seq = Kind::Sequence;
    assert_eq!(kinds, [(Kind::HotFile, 3), (seq, 1), (seq, 1)]);
    Ok(())
}

/// The tokens of `rows`' stored bodies, as mining counts them.
fn tokens(rows: &[Observation]) -> usize {
    rows.iter().map(|o| tokens_at_most(&o.body)).sum()
}

/// 3 big `cat` calls (1, 3, 5), each over 1/10 of the tokens of all,
/// 3 `ls` (2, 4, 6), all under 1/10, and 1 huge `echo` (7).
fn big_and_small() -> Res<Vec<Observation>> {
    let big = format!("cat {}", "src/lib.rs ".repeat(40));
    let sink = calls(&[(1, "s", &big), (3, "s", &big), (5, "s", &big)])?;
    let small = calls(&[(2, "s", "ls"), (4, "s", "ls"), (6, "s", "ls")])?;
    let huge = bash(7, "s", &format!("echo {}", "word ".repeat(400)))?;
    let rows = [sink.clone(), small.clone(), vec![huge]].concat();
    let total = tokens(&rows);
    assert!(
        tokens(&sink).saturating_mul(10) >= total,
        "sink under 1/10 of {total}"
    );
    assert!(
        tokens(&small).saturating_mul(10) < total,
        "ls over 1/10 of {total}"
    );
    Ok(rows)
}

#[test]
fn a_pattern_with_a_tenth_of_the_tokens_is_a_token_sink() -> Test {
    let found = findings(&big_and_small()?, 3);
    let sinks = of(&found, Kind::TokenSink);
    let [only] = sinks.as_slice() else {
        return Err(format!("not 1 token sink: {found:?}").into());
    };
    assert_eq!((only.count, only.refs.clone()), (3, refs(&[1, 3, 5])?));
    let head = "token sink `Bash` `cat src/lib.rs src/lib.rs";
    assert!(only.summary.starts_with(head), "{}", only.summary);
    Ok(())
}

#[test]
fn a_sink_of_another_tool_is_named_by_its_paths() -> Test {
    let read = |n| row(n, "s", "Read", r#"{"file_path":"src/lib.rs"}"#);
    let found = findings(&[read(1)?, read(2)?, read(3)?], 3);
    let sinks: Vec<_> = of(&found, Kind::TokenSink)
        .iter()
        .map(|f| (f.count, f.summary.clone()))
        .collect();
    let want = "token sink `Read` `src/lib.rs`: >= 1/10 of tool call tokens";
    assert_eq!(sinks, [(3, want.to_owned())]);
    Ok(())
}

#[test]
fn a_tool_naming_no_command_or_path_is_1_pattern() -> Test {
    let search = |n, q: &str| {
        let input = serde_json::json!({ "query": q }).to_string();
        row(n, "s", "WebSearch", &input)
    };
    let rows = [search(1, "a")?, search(2, "b")?, search(3, "c")?];
    let found = findings(&rows, 3);
    let sinks: Vec<_> = of(&found, Kind::TokenSink)
        .iter()
        .map(|f| (f.refs.clone(), f.summary.clone()))
        .collect();
    let want = "token sink `WebSearch`: >= 1/10 of tool call tokens";
    assert_eq!(sinks, [(refs(&[1, 2, 3])?, want.to_owned())]);
    Ok(())
}

#[test]
fn a_sink_called_too_few_times_is_no_finding() -> Test {
    let big = format!("cat {}", "src/lib.rs ".repeat(40));
    let found = findings(&calls(&[(1, "s", &big), (2, "s", &big)])?, 3);
    assert!(of(&found, Kind::TokenSink).is_empty(), "{found:?}");
    Ok(())
}
