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
    let wanted = |r: &&Row| {
        r.class() == class && (only.is_empty() || only.contains(&r.fixture))
    };
    let pairs: Vec<(&Row, &Bench)> = rows
        .iter()
        .filter(wanted)
        .filter_map(|r| Some((r, now.iter().find(|b| measures(b, r))?)))
        .collect();
    let regressions = pairs.iter().filter_map(|(r, b)| regression(r, b));
    Verdict {
        regressions: regressions.collect(),
        compared: pairs.len(),
    }
}

fn measures(b: &Bench, r: &Row) -> bool {
    b.suite == r.suite && b.fixture == r.fixture
}

/// The line for `b` when it regressed against `r`.
fn regression(r: &Row, b: &Bench) -> Option<String> {
    let limit = r.median_ms * (1.0 + r.tolerance);
    (b.median_ms > limit).then(|| {
        format!(
            "{}/{}: median {:.3} ms > {:.3} ms, baseline {:.3} ms x (1 + \
             {}) of {}; recount: {}",
            r.suite,
            r.fixture,
            b.median_ms,
            limit,
            r.median_ms,
            r.tolerance,
            r.date,
            r.recount
        )
    })
}
