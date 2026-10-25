//! criterion 0.8's output (benches R1): per bench
//! `<suite>/<fixture>/new/estimates.json`, the median's point estimate,
//! and `new/sample.json`, the raw samples a p95 is taken from.

#[cfg(test)]
mod tests;

use std::path::Path;

/// 1 bench of a criterion run, in milliseconds a call.
#[derive(Debug, Clone, PartialEq)]
pub struct Bench {
    pub suite: String,
    pub fixture: String,
    pub median_ms: f64,
    pub p95_ms: f64,
    /// When criterion wrote it, `YYYY-MM-DD`.
    pub date: String,
}

/// The date a file was written, `YYYY-MM-DD`.
pub type DateOf = fn(&Path) -> Option<String>;

/// The median's point estimate of `estimates.json`, in ms.
#[must_use]
pub fn median_ms(json: &str) -> Option<f64> {
    let _ = json;
    None
}

/// The 95th percentile, by nearest rank, of 1 call's time over the
/// samples of `sample.json` (each sample: `times` / `iters`), in ms.
#[must_use]
pub fn p95_ms(json: &str) -> Option<f64> {
    let _ = json;
    None
}

/// Every bench under the criterion dir `dir` with both files readable,
/// sorted by suite then fixture, each dated by `date_of`; a dir that is
/// not a bench is skipped.
#[must_use]
pub fn read(dir: &Path, date_of: DateOf) -> Vec<Bench> {
    let _ = (dir, date_of);
    Vec::new()
}
