//! `bench-record` (benches T4): a criterion run turned into baseline
//! rows. A measured bench replaces its row on the same machine class;
//! every other row is kept, so a partial run re-records only itself.

#[cfg(test)]
mod tests;

use crate::baseline::Row;
use crate::criterion::Bench;

/// A suite's tolerance until one is tuned in the baseline by hand.
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
    let _ = text;
    Vec::new()
}

/// The baseline after `benches` ran on `host`.
#[must_use]
pub fn rows(
    index: &[(String, Option<String>)],
    benches: &[Bench],
    host: &Host,
    before: &[Row],
) -> Vec<Row> {
    let _ = (index, benches, host, before);
    Vec::new()
}

/// The command that measures 1 row again.
#[must_use]
pub fn recount(suite: &str, fixture: &str) -> String {
    let _ = (suite, fixture);
    String::new()
}

/// `100k-p520` -> (100000, 520).
#[must_use]
pub fn fixture_size(label: &str) -> Option<(u64, u64)> {
    let _ = label;
    None
}
