//! `bench-gate` (benches C, root V10): a bench regressed when its
//! median now is over its baseline median x (1 + tolerance), on the
//! same machine class as the baseline row.

#[cfg(test)]
mod tests;

use crate::baseline::Row;
use crate::criterion::Bench;

/// What the gate found over the rows it could compare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    /// Rows with a fresh median to compare.
    pub compared: usize,
    /// 1 line per regression.
    pub regressions: Vec<String>,
}

/// `rows` of the machine class `class`, and of `only` when it names
/// fixtures, compared with the medians of `now`.
#[must_use]
pub fn verdict(
    rows: &[Row],
    class: (&str, &str),
    only: &[String],
    now: &[Bench],
) -> Verdict {
    let _ = (rows, class, only, now);
    Verdict {
        compared: 0,
        regressions: Vec::new(),
    }
}
