//! `bench-record` (benches T4): a criterion run turned into baseline
//! rows. A measured bench replaces its row on the same machine class;
//! every other row is kept, so a partial run re-records only itself.

#[cfg(test)]
mod tests;

use crate::baseline::{Row, round};
use crate::criterion::Bench;

/// A suite's tolerance until one is tuned in the baseline by hand: the
/// push run takes 10 samples, so a median moves by a quarter on a busy
/// machine without any code change.
pub const TOLERANCE: f64 = 0.5;

/// The machine a run was measured on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Host {
    pub machine: String,
    pub os: String,
}

/// The suite index the bench run writes: `(suite, target)` in order,
/// `-` for no target.
#[must_use]
pub fn index(text: &str) -> Vec<(String, Option<String>)> {
    let fields = text.lines().filter_map(|l| l.split_once('\t'));
    let target = |t: &str| (t != "-").then(|| t.to_owned());
    fields.map(|(s, t)| (s.to_owned(), target(t))).collect()
}

/// The baseline after `benches` ran on `host`: their rows, then every
/// row of `before` not measured again, sorted by suite in `index`
/// order, then fixture size and partitions. A bench of a suite not in
/// `index` is left out.
#[must_use]
pub fn rows(
    index: &[(String, Option<String>)],
    benches: &[Bench],
    host: &Host,
    before: &[Row],
) -> Vec<Row> {
    let fresh = benches.iter().filter_map(|b| {
        let (_, target) = index.iter().find(|(s, _)| *s == b.suite)?;
        Some(row(b, target.clone(), host, tolerance(before, &b.suite)))
    });
    let mut rows: Vec<Row> = fresh.collect();
    let kept = before.iter().filter(|r| {
        index.iter().any(|(s, _)| *s == r.suite)
            && !rows.iter().any(|n| same(n, r))
    });
    rows.extend(kept.cloned().collect::<Vec<_>>());
    rows.sort_by_key(|r| order(index, r));
    rows
}

fn row(b: &Bench, target: Option<String>, host: &Host, tolerance: f64) -> Row {
    Row {
        recount: recount(&b.suite, &b.fixture),
        suite: b.suite.clone(),
        fixture: b.fixture.clone(),
        target,
        median_ms: round(b.median_ms),
        p95_ms: round(b.p95_ms),
        tolerance,
        machine: host.machine.clone(),
        os: host.os.clone(),
        date: b.date.clone(),
    }
}

/// The command that measures 1 row again.
#[must_use]
pub fn recount(suite: &str, fixture: &str) -> String {
    format!("cargo bench -p chertmem --bench latency -- '^{suite}/{fixture}$'")
}

/// The suite's tolerance in `before`, else [`TOLERANCE`].
fn tolerance(before: &[Row], suite: &str) -> f64 {
    let tuned = before.iter().find(|r| r.suite == suite);
    tuned.map_or(TOLERANCE, |r| r.tolerance)
}

/// The same bench on the same machine class.
fn same(a: &Row, b: &Row) -> bool {
    a.suite == b.suite && a.fixture == b.fixture && a.class() == b.class()
}

type Order = (usize, (u64, u64), String, String);

fn order(index: &[(String, Option<String>)], r: &Row) -> Order {
    let at = index.iter().position(|(s, _)| *s == r.suite);
    let size = fixture_size(&r.fixture).unwrap_or((u64::MAX, u64::MAX));
    (
        at.unwrap_or(usize::MAX),
        size,
        r.machine.clone(),
        r.os.clone(),
    )
}

/// `100k-p520` -> (100000, 520).
#[must_use]
pub fn fixture_size(label: &str) -> Option<(u64, u64)> {
    let (size, parts) = label.split_once("-p")?;
    let (digits, unit) = match size.strip_suffix('M') {
        Some(d) => (d, 1_000_000),
        None => size.strip_suffix('k').map_or((size, 1), |d| (d, 1000)),
    };
    let n = digits.parse::<u64>().ok()?.checked_mul(unit)?;
    Some((n, parts.parse().ok()?))
}
