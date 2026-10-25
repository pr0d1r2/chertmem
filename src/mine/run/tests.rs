use std::fs;
use std::path::Path;

use rusqlite::Connection;
use serde_json::json;

use super::mine;
use crate::mine::tests::{Res, Scratch, Test};
use crate::mine::{Finding, Kind};
use crate::store::{Filter, Lookup, Store};

/// 8 days in ms: past the default 7d bucket, so a 2nd partition.
const WEEK_ON: u64 = 8 * 24 * 3_600_000;

/// `cargo fmt` then `cargo test` 3 times in session `s`, from `at` on.
fn three_rounds(scratch: &Scratch, at: u64) -> Test {
    for step in 0..6_u64 {
        let command = if step % 2 == 0 {
            "cargo fmt"
        } else {
            "cargo test"
        };
        let n = at.checked_add(step).ok_or("time out of range")?;
        scratch.bash(n, "s", command)?;
    }
    Ok(())
}

fn sequences(found: &[Finding]) -> Vec<(usize, &str)> {
    let seqs = found.iter().filter(|f| f.kind == Kind::Sequence);
    seqs.map(|f| (f.count, f.summary.as_str())).collect()
}

const PAIR: &str = "`cargo fmt` then `cargo test`";

fn seal_all(store: &mut Store) -> Test {
    for p in store.partitions(&Filter::default())? {
        store.seal(&p.path)?;
    }
    Ok(())
}

fn ledger(scratch: &Scratch) -> Res<Connection> {
    Ok(Connection::open(scratch.data().join("mine/findings.db"))?)
}

#[test]
fn hook_events_give_a_sequence_whose_refs_resolve() -> Test {
    let scratch = Scratch::new("resolve")?;
    three_rounds(&scratch, 0)?;
    let mut store = scratch.store()?;
    let found = mine(&mut store, &scratch.config, None)?;
    assert_eq!(sequences(&found), [(3, PAIR)]);
    for f in &found {
        assert!(f.refs.len() >= 3 && f.refs.len() == f.count, "{f:?}");
        let got = store.get(&f.refs)?;
        assert!(got.iter().all(|l| matches!(l, Lookup::Found(_))), "{got:?}");
    }
    Ok(())
}

/// The recorded failure (src/hook R5) of `false`, stored by `src/hook`
/// at each `(at, exit code)`.
fn fail_false(scratch: &Scratch, codes: &[(u64, u8)]) -> Test {
    let input = json!({"command": "false", "description": "Run false"});
    for (at, code) in codes {
        let error = format!("Exit code {code}");
        scratch.failure(*at, "s", ("Bash", &input, &error))?;
    }
    Ok(())
}

/// `false` failing with exit code 1 3 times and with 2 once.
#[test]
fn hook_failures_give_a_failure_whose_refs_resolve() -> Test {
    let scratch = Scratch::new("failure")?;
    fail_false(&scratch, &[(0, 1), (1, 1), (2, 2), (3, 1)])?;
    let mut store = scratch.store()?;
    let found = mine(&mut store, &scratch.config, None)?;
    let mut fails = found.iter().filter(|f| f.kind == Kind::Failure);
    let only = fails.next().ok_or("no failure")?;
    assert_eq!(only.summary, "failing `Bash` `false`: `Exit code 1`");
    let got = store.get(&only.refs)?;
    let found_all = got.iter().all(|l| matches!(l, Lookup::Found(_)));
    let more = fails.count();
    assert!(got.len() == 3 && found_all && more == 0, "{found:?}");
    Ok(())
}

#[test]
fn the_same_store_gives_the_same_findings_sealed_or_not() -> Test {
    let scratch = Scratch::new("same")?;
    three_rounds(&scratch, 0)?;
    let mut store = scratch.store()?;
    let hot = mine(&mut store, &scratch.config, None)?;
    assert!(!hot.is_empty());
    assert_eq!(mine(&mut store, &scratch.config, None)?, hot);
    seal_all(&mut store)?;
    assert_eq!(mine(&mut store, &scratch.config, None)?, hot);
    assert_eq!(mine(&mut store, &scratch.config, None)?, hot);
    Ok(())
}

#[test]
fn a_sealed_partition_is_mined_once() -> Test {
    let scratch = Scratch::new("once")?;
    three_rounds(&scratch, 0)?;
    let mut store = scratch.store()?;
    seal_all(&mut store)?;
    mine(&mut store, &scratch.config, None)?;
    ledger(&scratch)?.execute("UPDATE findings SET summary = 'kept'", [])?;
    let found = mine(&mut store, &scratch.config, None)?;
    assert_eq!(sequences(&found), [(3, "kept")]);
    Ok(())
}

#[test]
fn a_new_threshold_mines_a_sealed_partition_again() -> Test {
    let scratch = Scratch::new("threshold")?;
    three_rounds(&scratch, 0)?;
    let mut store = scratch.store()?;
    seal_all(&mut store)?;
    mine(&mut store, &scratch.config, None)?;
    let mut config = scratch.config.clone();
    config.mining.min_occurrences = 2;
    let found = mine(&mut store, &config, None)?;
    assert_eq!(
        sequences(&found),
        [(3, PAIR), (2, "`cargo test` then `cargo fmt`")]
    );
    Ok(())
}

#[test]
fn a_hot_partition_is_mined_on_demand_and_never_kept() -> Test {
    let scratch = Scratch::new("hot")?;
    scratch.bash(0, "s", "ls")?;
    let mut store = scratch.store()?;
    assert!(mine(&mut store, &scratch.config, None)?.is_empty());
    let kept: i64 =
        ledger(&scratch)?
            .query_row("SELECT count(*) FROM mined", [], |r| r.get(0))?;
    assert_eq!(kept, 0);
    three_rounds(&scratch, 10)?;
    let mut store = scratch.store()?;
    let found = mine(&mut store, &scratch.config, None)?;
    assert_eq!(sequences(&found), [(3, PAIR)]);
    Ok(())
}

#[test]
fn findings_merge_by_id_across_partitions() -> Test {
    let scratch = Scratch::new("merge")?;
    three_rounds(&scratch, 0)?;
    three_rounds(&scratch, WEEK_ON)?;
    let mut store = scratch.store()?;
    assert_eq!(store.partitions(&Filter::default())?.len(), 2);
    let found = mine(&mut store, &scratch.config, None)?;
    assert_eq!(sequences(&found), [(6, PAIR)]);
    let refs = found.first().map(|f| f.refs.clone()).unwrap_or_default();
    let mut sorted = refs.clone();
    sorted.sort();
    assert_eq!(refs, sorted);
    Ok(())
}

#[test]
fn since_skips_earlier_partitions() -> Test {
    let scratch = Scratch::new("since")?;
    three_rounds(&scratch, 0)?;
    three_rounds(&scratch, WEEK_ON)?;
    let mut store = scratch.store()?;
    let later = super::super::tests::NOW.checked_add(WEEK_ON);
    let found = mine(&mut store, &scratch.config, later)?;
    assert_eq!(sequences(&found), [(3, PAIR)]);
    Ok(())
}

#[test]
fn hot_files_come_from_the_paths_tools_name() -> Test {
    let scratch = Scratch::new("hotfile")?;
    for at in 0..3 {
        let input = json!({ "file_path": "src/lib.rs" });
        scratch.tool(at, "s", ("Read", &input))?;
    }
    let mut store = scratch.store()?;
    let found = mine(&mut store, &scratch.config, None)?;
    let hot = found.iter().filter(|f| f.kind == Kind::HotFile);
    let hot: Vec<_> = hot.map(|f| (f.kind, f.count)).collect();
    assert_eq!(hot, [(Kind::HotFile, 3)]);
    Ok(())
}

#[test]
fn token_sinks_come_from_the_stored_bodies() -> Test {
    let scratch = Scratch::new("tokensink")?;
    for at in 0..3 {
        let input = json!({ "file_path": "src/lib.rs" });
        scratch.tool(at, "s", ("Read", &input))?;
    }
    let mut store = scratch.store()?;
    seal_all(&mut store)?;
    let found = mine(&mut store, &scratch.config, None)?;
    let sinks = found.iter().filter(|f| f.kind == Kind::TokenSink);
    let sinks: Vec<_> = sinks.map(|f| (f.count, f.summary.as_str())).collect();
    let want = "token sink `Read` `src/lib.rs`: >= 1/10 of tool call tokens";
    assert_eq!(sinks, [(3, want)]);
    Ok(())
}

#[test]
fn mining_off_finds_nothing_and_writes_nothing() -> Test {
    let scratch = Scratch::new("off")?;
    three_rounds(&scratch, 0)?;
    let mut config = scratch.config.clone();
    config.mining.enabled = false;
    let mut store = scratch.store()?;
    assert!(mine(&mut store, &config, None)?.is_empty());
    assert!(!scratch.data().join("mine").exists());
    Ok(())
}

/// Every path under `dir`, recursively.
fn walk(dir: &Path, out: &mut Vec<String>) -> Test {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        out.push(path.display().to_string());
        if path.is_dir() {
            walk(&path, out)?;
        }
    }
    Ok(())
}

#[test]
fn nothing_is_written_under_rekall() -> Test {
    let scratch = Scratch::new("rekall")?;
    three_rounds(&scratch, 0)?;
    let mut store = scratch.store()?;
    seal_all(&mut store)?;
    mine(&mut store, &scratch.config, None)?;
    let mut all = Vec::new();
    walk(&scratch.dir, &mut all)?;
    assert!(all.iter().all(|p| !p.contains(".rekall")), "{all:?}");
    let mine_dir: Vec<_> = all.iter().filter(|p| p.contains("/mine")).collect();
    assert!(
        mine_dir.iter().all(|p| p.contains("data/mine")),
        "{mine_dir:?}"
    );
    Ok(())
}
