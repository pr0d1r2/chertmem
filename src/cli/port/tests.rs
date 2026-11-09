//! `chert export` and `chert import`: exit 0, a round trip keeps every
//! row; `--for rekall` writes the prompts only; an unreadable input is
//! exit 2 (V2).

use std::fs;

use rusqlite::Connection;

use super::super::tests::{SESSION, Sandbox, Test};

/// The tables cavemem 0.2.1's importer reads, empty (src/import R).
const CAVEMEM: &str = "
CREATE TABLE schema_version (version INTEGER PRIMARY KEY);
CREATE TABLE sessions (id TEXT PRIMARY KEY, ide TEXT NOT NULL, cwd TEXT,
  started_at INTEGER NOT NULL, ended_at INTEGER, metadata TEXT);
CREATE TABLE observations (id INTEGER PRIMARY KEY, session_id TEXT,
  kind TEXT, content TEXT, compressed INTEGER, intensity TEXT,
  ts INTEGER, metadata TEXT);
CREATE TABLE summaries (id INTEGER PRIMARY KEY, session_id TEXT,
  scope TEXT, content TEXT, compressed INTEGER, intensity TEXT,
  ts INTEGER);
INSERT INTO schema_version(version) VALUES (2);
";

#[test]
fn export_then_import_keeps_every_row() -> Test {
    let (from, to) = (Sandbox::new("export")?, Sandbox::new("import")?);
    from.hook("post-tool-use")?;
    let out = from.file("out.jsonl");
    let path = out.to_string_lossy().into_owned();
    let ran = from.run(&["export", &path]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert!(ran.out.starts_with("exported 1 observations to "));
    assert_eq!(fs::read_to_string(&out)?.lines().count(), 1);
    let ran = to.run(&["import", &path]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert!(ran.out.starts_with("imported 1 lines from "), "{ran:?}");
    let timeline = |s: &Sandbox| s.run(&["timeline", SESSION]).out;
    assert_eq!(timeline(&to), timeline(&from));
    Ok(())
}

/// Whether rekall V52 sniffs `line` as a chert export: `kind`, a `ref`
/// starting `chert:`, and `ts`.
fn sniffed(line: &serde_json::Value) -> bool {
    use serde_json::Value;
    let field = |k: &str| line.get(k).and_then(Value::as_str);
    field("kind").is_some()
        && field("ref").is_some_and(|r| r.starts_with("chert:"))
        && line.get("ts").and_then(Value::as_u64).is_some()
}

/// Every line of the JSONL file at `path`.
fn json_lines(
    path: &std::path::Path,
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    Ok(text
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?)
}

#[test]
fn export_for_rekall_writes_prompts_rekall_sniffs() -> Test {
    let s = Sandbox::new("export-rekall")?;
    s.hook("post-tool-use")?;
    s.hook("user-prompt-submit")?;
    let out = s.file("out.jsonl");
    let ran = s.run(&["export", &out.to_string_lossy(), "--for", "rekall"]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert!(ran.out.starts_with("exported 1 prompts for rekall to "));
    let lines = json_lines(&out)?;
    let prompt = serde_json::Value::from("user-prompt-submit");
    assert!(lines.iter().all(|l| l.get("kind") == Some(&prompt)));
    assert!(lines.len() == 1 && lines.iter().all(sniffed), "{lines:?}");
    Ok(())
}

#[test]
fn export_to_an_unwritable_path_is_exit_2() -> Test {
    let s = Sandbox::new("export-unwritable")?;
    let out = s.file("no-dir/out.jsonl");
    let ran = s.run(&["export", &out.to_string_lossy()]);
    assert_eq!(ran.code, 2, "{ran:?}");
    assert!(ran.err.contains("cannot write"), "{ran:?}");
    Ok(())
}

#[test]
fn import_of_a_missing_or_bad_file_is_exit_2() -> Test {
    let s = Sandbox::new("import-bad")?;
    let bad = s.file("bad.jsonl");
    fs::write(&bad, "not json\n")?;
    let missing = s.run(&["import", "no-such.jsonl"]);
    assert!(missing.err.contains("cannot read"), "{missing:?}");
    assert_eq!(missing.code, 2);
    assert_eq!(s.run(&["import", &bad.to_string_lossy()]).code, 2);
    let cave = s.run(&["import", &bad.to_string_lossy(), "--from", "cavemem"]);
    assert_eq!(cave.code, 2, "{cave:?}");
    Ok(())
}

#[test]
fn import_from_cavemem_counts_what_it_stored() -> Test {
    let s = Sandbox::new("import-cavemem")?;
    let db = s.file("cavemem.db");
    Connection::open(&db)?.execute_batch(CAVEMEM)?;
    let ran = s.run(&["import", &db.to_string_lossy(), "--from", "cavemem"]);
    assert_eq!(ran.code, 0, "{ran:?}");
    let want = "imported 0 observations, 0 summaries \
                (0 without a cwd, under this project)\n";
    assert_eq!(ran.out, want);
    Ok(())
}
