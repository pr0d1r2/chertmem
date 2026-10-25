#[cfg(feature = "bpe")]
mod corpus;

use std::collections::BTreeSet;
use std::fs;

use serde_json::Value;

use super::{HEADER, budget, cut, recent, tokens_at_most};
use crate::hook::tests::{NOW, Res, Scratch, Test, fixture};
use crate::hook::{Kind, Payload};
use crate::store::{Filter, Scope, Store, Summary};

fn json(text: &serde_json::Value) -> Vec<u8> {
    text.to_string().into_bytes()
}

/// Session `session` at `at`: 1 turn saying `said`, then its end.
fn past(scratch: &Scratch, session: &str, at: u64, said: &str) {
    let stop = serde_json::json!({
        "session_id": session, "last_assistant_message": said});
    scratch.run_at(Kind::Stop, &json(&stop), at);
    let end = serde_json::json!({"session_id": session, "reason": "other"});
    scratch.run_at(Kind::SessionEnd, &json(&end), at.saturating_add(1));
}

/// `count` past sessions `p0`, `p1`, ... 10 ms apart, oldest first.
fn history(scratch: &Scratch, count: u64, said: &str) {
    for n in 0..count {
        let at = NOW.saturating_add(n.saturating_mul(10));
        past(scratch, &format!("p{n}"), at, &format!("{said} {n}"));
    }
}

fn start(source: &str) -> Vec<u8> {
    json(&serde_json::json!({"session_id": "now", "source": source}))
}

/// The context hook `kind` printed, checked to be the only output.
fn context(out: Option<String>, kind: Kind) -> Res<String> {
    let value: Value = serde_json::from_str(&out.ok_or("no stdout")?)?;
    let object = value.as_object().ok_or("not an object")?;
    assert_eq!(object.keys().collect::<Vec<_>>(), ["hookSpecificOutput"]);
    let inner = object.get("hookSpecificOutput").ok_or("no output")?;
    let inner = inner.as_object().ok_or("not an object")?;
    let keys: Vec<_> = inner.keys().collect();
    assert_eq!(keys, ["additionalContext", "hookEventName"]);
    let name = inner.get("hookEventName").and_then(Value::as_str);
    assert_eq!(name, Some(kind.event_name()));
    let text = inner.get("additionalContext").and_then(Value::as_str);
    Ok(text.ok_or("no context")?.to_owned())
}

fn at_start(scratch: &Scratch, source: &str) -> Option<String> {
    let at = NOW.saturating_add(1_000);
    scratch
        .run_at(Kind::SessionStart, &start(source), at)
        .stdout
}

#[test]
fn startup_injects_the_last_3_other_sessions_newest_first() -> Test {
    let mut scratch = Scratch::new("inject")?;
    scratch.config.inject.max_tokens = 10_000;
    history(&scratch, 5, "Fixed the parser");
    past(&scratch, "now", NOW.saturating_add(500), "this session");
    let text = context(at_start(&scratch, "startup"), Kind::SessionStart)?;
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.first(), Some(&HEADER));
    assert_eq!(lines.len(), 4, "{text}");
    for (line, n) in lines.iter().skip(1).zip([4, 3, 2]) {
        assert!(line.starts_with("- chert:"), "{line}");
        assert!(line.ends_with(&format!("parser {n}")), "{line}");
    }
    assert!(!text.contains("this session"));
    Ok(())
}

/// 30 days in ms: past the default 7d bucket.
const MONTH: u64 = 30 * 86_400_000;

/// Overwrite the partition holding `ts` with bytes `SQLite` refuses, so
/// any read of it fails.
fn break_partition_at(scratch: &Scratch, ts: u64) -> Test {
    let filter = Filter {
        project: None,
        since: Some(ts),
        until: ts.checked_add(1),
    };
    let found = scratch.store()?.partitions(&filter)?;
    let part = found.first().ok_or("no partition")?;
    let file = scratch.data().join(&part.path);
    for end in ["-wal", "-shm"] {
        let _ = fs::remove_file(format!("{}{end}", file.display()));
    }
    Ok(fs::write(file, "not a database, never opened")?)
}

#[test]
fn injection_reads_only_the_newest_partitions_it_needs() -> Test {
    let mut scratch = Scratch::new("injectnewest")?;
    scratch.config.inject.max_tokens = 10_000;
    let old = NOW.saturating_sub(MONTH);
    past(&scratch, "old", old, "Long ago");
    history(&scratch, 3, "Fixed the parser");
    break_partition_at(&scratch, old)?;
    let out = at_start(&scratch, "startup");
    let text = context(out, Kind::SessionStart)?;
    assert_eq!(text.lines().count(), 4, "{text}");
    assert!(!text.contains("Long ago"), "{text}");
    Ok(())
}

/// The pick as made before src/hook T11: every partition read, then
/// newest first by time then id, 1 per other session.
fn every(store: &mut Store, session: &str) -> Res<Vec<Summary>> {
    let all = store.summaries(None, Scope::Session)?;
    let mut seen = BTreeSet::new();
    let newest = all.into_iter().rev().filter(|s| s.session != session);
    let fresh = newest.filter(|s| seen.insert(s.session.clone()));
    Ok(fresh.take(3).collect())
}

/// 3 weeks of session summaries, 4 sharing each time, of `sessions`
/// sessions that recur across weeks: ties within and across shard
/// files.
fn tied_history(store: &mut Store, sessions: u64) -> Test {
    for n in 0..24_u64 {
        let (week, k) = (n / 8, n % 8);
        let back = week.saturating_mul(MONTH / 4).saturating_add(k / 4);
        let summary = Summary {
            ts: NOW.saturating_sub(back),
            session: format!("s{}", n.checked_rem(sessions).unwrap_or(0)),
            scope: Scope::Session,
            ide: "claude-code".to_owned(),
            body: format!("summary {n}"),
        };
        store.append_summary(&summary)?;
    }
    Ok(())
}

#[test]
fn reading_newest_partitions_first_picks_as_reading_every_one_did() -> Test {
    let mut scratch = Scratch::new("injectties")?;
    scratch.config.partition.shards = 4;
    let mut store = scratch.store()?;
    tied_history(&mut store, 6)?;
    let parts = store.partitions(&Filter::default())?;
    assert!(parts.len() > 3, "{parts:?}");
    for own in ["", "s0", "s1", "s2", "s3", "s4", "s5", "other"] {
        let picked = recent(&mut store, own)?;
        assert_eq!(picked, every(&mut store, own)?, "{own}");
        assert_eq!(picked.len(), 3, "{own}");
    }
    Ok(())
}

#[test]
fn fewer_than_3_other_sessions_are_picked_from_every_partition() -> Test {
    let mut scratch = Scratch::new("injectfew")?;
    scratch.config.partition.shards = 2;
    let mut store = scratch.store()?;
    tied_history(&mut store, 3)?;
    for (own, count) in [("s0", 2), ("s2", 2), ("other", 3)] {
        let picked = recent(&mut store, own)?;
        assert_eq!(picked, every(&mut store, own)?, "{own}");
        assert_eq!(picked.len(), count, "{own}");
    }
    Ok(())
}

#[test]
fn each_ref_resolves_to_its_sessions_end() -> Test {
    let mut scratch = Scratch::new("injectref")?;
    scratch.config.inject.max_tokens = 10_000;
    history(&scratch, 1, "Done");
    let text = context(at_start(&scratch, "startup"), Kind::SessionStart)?;
    let word = text.lines().nth(1).and_then(|l| l.split(' ').nth(1));
    let reference = crate::store::Ref::parse(word.ok_or("no ref")?)?;
    let found = scratch.store()?.get(&[reference])?;
    let crate::store::Lookup::Found(obs) = found.first().ok_or("none")? else {
        return Err("dropped".into());
    };
    assert_eq!(
        (obs.session.as_str(), obs.kind.as_str()),
        ("p0", "session-end")
    );
    Ok(())
}

#[test]
fn a_summary_without_its_end_is_injected_without_a_ref() -> Test {
    let mut scratch = Scratch::new("injectnoref")?;
    scratch.config.inject.max_tokens = 10_000;
    let summary = Summary {
        ts: NOW,
        session: "imported".to_owned(),
        scope: Scope::Session,
        ide: "claude-code".to_owned(),
        body: "Moved the build to nix".to_owned(),
    };
    scratch.store()?.append_summary(&summary)?;
    let text = context(at_start(&scratch, "startup"), Kind::SessionStart)?;
    assert_eq!(text, format!("{HEADER}\n- Moved the build to nix"));
    Ok(())
}

#[test]
fn only_the_first_line_of_a_summary_is_injected() -> Test {
    let mut scratch = Scratch::new("injectgist")?;
    scratch.config.inject.max_tokens = 10_000;
    for (n, said) in ["First turn here", "Second turn body"].iter().enumerate()
    {
        let stop = serde_json::json!({
            "session_id": "p", "last_assistant_message": said});
        let at = NOW.saturating_add(u64::try_from(n)?);
        scratch.run_at(Kind::Stop, &json(&stop), at);
    }
    let end = json(&serde_json::json!({"session_id": "p"}));
    scratch.run_at(Kind::SessionEnd, &end, NOW.saturating_add(2));
    let text = context(at_start(&scratch, "startup"), Kind::SessionStart)?;
    assert!(text.contains("First turn here") && !text.contains("Second"));
    Ok(())
}

#[test]
fn resume_compact_and_clear_inject_nothing() -> Test {
    let scratch = Scratch::new("injectresume")?;
    history(&scratch, 2, "Work");
    for source in ["resume", "compact", "clear", ""] {
        assert_eq!(at_start(&scratch, source), None, "{source}");
    }
    let bare = br#"{"session_id":"now"}"#;
    assert_eq!(scratch.run(Kind::SessionStart, bare).stdout, None);
    Ok(())
}

#[test]
fn no_other_session_injects_nothing() -> Test {
    let scratch = Scratch::new("injectempty")?;
    let out = scratch.run(Kind::SessionStart, &fixture(Kind::SessionStart)?);
    assert_eq!((out.stdout, out.note), (None, None));
    Ok(())
}

#[test]
fn prompts_inject_only_when_on_prompt_is_set() -> Test {
    let mut scratch = Scratch::new("injectprompt")?;
    history(&scratch, 2, "Work");
    let prompt = fixture(Kind::UserPromptSubmit)?;
    let at = NOW.saturating_add(1_000);
    assert_eq!(
        scratch.run_at(Kind::UserPromptSubmit, &prompt, at).stdout,
        None
    );
    scratch.config.inject.on_prompt = true;
    let out = scratch.run_at(Kind::UserPromptSubmit, &prompt, at);
    let text = context(out.stdout, Kind::UserPromptSubmit)?;
    assert!(text.starts_with(HEADER));
    Ok(())
}

/// A payload whose session started for `source`.
fn from(source: &str) -> Payload {
    Payload {
        session: "s".to_owned(),
        identity: None,
        body: String::new(),
        paths: Vec::new(),
        source: Some(source.to_owned()),
        cwd: None,
    }
}

#[test]
fn only_startup_and_opted_in_prompts_have_a_budget() -> Test {
    let scratch = Scratch::new("budget")?;
    let mut config = scratch.config.clone();
    let start = from("startup");
    assert_eq!(budget(Kind::SessionStart, &start, &config), Some(300));
    let resume = from("resume");
    assert_eq!(budget(Kind::SessionStart, &resume, &config), None);
    assert_eq!(budget(Kind::UserPromptSubmit, &start, &config), None);
    for kind in [Kind::PostToolUse, Kind::Stop, Kind::SessionEnd] {
        assert_eq!(budget(kind, &start, &config), None, "{kind:?}");
    }
    config.inject.on_prompt = true;
    assert_eq!(budget(Kind::UserPromptSubmit, &start, &config), Some(300));
    Ok(())
}

#[test]
fn injected_text_stays_within_budget_at_every_size() -> Test {
    let long = "Refactored the store and the hook. ".repeat(40);
    for (n, count) in [0, 1, 3, 6].into_iter().enumerate() {
        let mut scratch = Scratch::new(&format!("injectsize-{n}"))?;
        history(&scratch, count, &long);
        for max in [0, 10, 60, 100, 300, 1_000, 5_000] {
            scratch.config.inject.max_tokens = max;
            let Some(out) = at_start(&scratch, "startup") else {
                continue;
            };
            let text = context(Some(out), Kind::SessionStart)?;
            let bound = usize::try_from(max)?;
            assert!(tokens_at_most(&text) <= bound, "{max}: {text}");
        }
    }
    Ok(())
}

#[test]
fn cut_keeps_whole_lines_until_one_does_not_fit() {
    let lines = ["- a".to_owned(), "- bbbb".to_owned(), "- c".to_owned()];
    let room = |text: &str| u32::try_from(tokens_at_most(text)).unwrap_or(0);
    let first = format!("{HEADER}\n- a");
    assert_eq!(cut(&lines, room(&first)), Some(first));
    let all = format!("{HEADER}\n- a\n- bbbb\n- c");
    assert_eq!(cut(&lines, u32::MAX), Some(all));
    assert_eq!(cut(&lines, room(HEADER)), None);
    assert_eq!(cut(&[], u32::MAX), None);
}

/// The default count is the memory tokenizer (T14, B1): the header is
/// its 12 o200k tokens, where bytes / 4 counted 11.
#[cfg(not(feature = "bpe"))]
#[test]
fn the_default_count_is_the_memory_tokenizer() {
    for text in ["", "chert", "chertmem!", HEADER, "- chert:01MH94JKC0 a"] {
        assert_eq!(tokens_at_most(text), super::estimate::tokens(text));
    }
    assert_eq!(tokens_at_most(HEADER), 12);
    assert_eq!(HEADER.len().div_ceil(4), 11);
}

/// With feature `bpe` the count is exact o200k.
#[cfg(feature = "bpe")]
#[test]
fn the_bpe_count_is_exact_o200k() {
    assert_eq!(tokens_at_most(""), 0);
    assert_eq!(tokens_at_most("hello world"), 2);
    assert_eq!(tokens_at_most(&"a".repeat(64)), 8);
}
