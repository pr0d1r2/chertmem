//! The bench verbs: `bench-record` (benches T4). Paths are the
//! repository's own; criterion's output is read from `target/criterion`.

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use crate::baseline::{self, Row};
use crate::{criterion, host, record};

pub(super) const BASELINE: &str = "benches/baseline.toml";
/// The suite index the bench run writes beside its group dirs.
pub(super) const INDEX: &str = "chert-suites.tsv";

/// The bench verb `verb` with `rest`; `None` when it is not one.
pub(super) fn run(verb: &str, rest: &[String], root: &Path) -> Option<u8> {
    match (verb, rest) {
        ("bench-record", []) => Some(recorded(root)),
        _ => None,
    }
}

fn criterion_dir(root: &Path) -> PathBuf {
    root.join("target").join("criterion")
}

/// `path` under `root` as text, or why not.
fn read(root: &Path, path: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(path))
        .map_err(|e| format!("cannot read {path}: {e}"))
}

/// The baseline's rows; none when the file does not exist yet.
fn rows(root: &Path) -> Result<Vec<Row>, String> {
    if !root.join(BASELINE).exists() {
        return Ok(Vec::new());
    }
    baseline::parse(&read(root, BASELINE)?)
        .map_err(|e| format!("{BASELINE}: {e}"))
}

/// The error printed under `verb`, then exit 2.
fn broken(verb: &str, why: &str) -> u8 {
    eprintln!("{verb}: {why}");
    2
}

/// `bench-record`: criterion's last run into `benches/baseline.toml`.
fn recorded(root: &Path) -> u8 {
    match record_rows(root) {
        Ok(rows) => write(root, BASELINE, &baseline::render(&rows)),
        Err(why) => broken("bench-record", &why),
    }
}

fn record_rows(root: &Path) -> Result<Vec<Row>, String> {
    let dir = criterion_dir(root);
    let index = std::fs::read_to_string(dir.join(INDEX)).map_err(|_| {
        format!("no {INDEX} under target/criterion; run `cargo bench` first")
    })?;
    let host = host::this().ok_or("cannot read the CPU model or OS")?;
    let benches = criterion::read(&dir, host::date_of);
    let index = record::index(&index);
    Ok(record::rows(&index, &benches, &host, &rows(root)?))
}

fn write(root: &Path, path: &str, text: &str) -> u8 {
    match std::fs::write(root.join(path), text) {
        Ok(()) => 0,
        Err(e) => broken(path, &format!("cannot write: {e}")),
    }
}
