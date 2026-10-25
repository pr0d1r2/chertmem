//! The bench verbs: `bench-record` (benches T4) and `bench-gate` (dev
//! T2). Paths are the repository's own; criterion's output is read from
//! `target/criterion`.

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use crate::baseline::{self, Row};
use crate::bench_gate::{self, Verdict};
use crate::{criterion, host, record};

pub(super) const BASELINE: &str = "benches/baseline.toml";
/// The suite index the bench run writes beside its group dirs.
pub(super) const INDEX: &str = "chert-suites.tsv";

/// The bench verb `verb` with `rest`; `None` when it is not one.
pub(super) fn run(verb: &str, rest: &[String], root: &Path) -> Option<u8> {
    match (verb, rest) {
        ("bench-gate", only) => Some(gate(root, only)),
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

/// The baseline and this machine, or why not.
fn gate_inputs(root: &Path) -> Result<(Vec<Row>, record::Host), String> {
    let host = host::this().ok_or("cannot read the CPU model or OS")?;
    Ok((rows(root)?, host))
}

/// `bench-gate [fixture...]`: the baseline rows of this machine class,
/// and of the named fixtures if any, against criterion's last run. A
/// machine the baseline has no row for holds: another CPU is not a
/// slowdown.
fn gate(root: &Path, only: &[String]) -> u8 {
    let (rows, host) = match gate_inputs(root) {
        Ok(inputs) => inputs,
        Err(why) => return broken("bench-gate", &why),
    };
    let class = (host.machine.as_str(), baseline::os_name(&host.os));
    if !rows.iter().any(|r| r.class() == class) {
        let (cpu, os) = class;
        eprintln!("bench-gate: no baseline row for {cpu}, {os}; held");
        return 0;
    }
    let now = criterion::read(&criterion_dir(root), host::date_of);
    verdict(&bench_gate::verdict(&rows, class, only, &now))
}

fn verdict(v: &Verdict) -> u8 {
    if v.compared == 0 {
        let why = "no baseline row has a median under target/criterion; \
                   run `cargo bench -p chertmem --bench latency` first";
        return broken("bench-gate", why);
    }
    for line in &v.regressions {
        eprintln!("bench-gate: {line}");
    }
    u8::from(!v.regressions.is_empty())
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
