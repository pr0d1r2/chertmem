//! The 1 scoring function for cached and disk rows (src/search C, V1)
//! and the total order of hits (V4). A row's score depends on its body,
//! the query's terms and the global stats only, never on the partition
//! or cache it came from, so merging per-partition bests equals ranking
//! 1 table of the same rows (V2).

#[cfg(test)]
mod tests;

use std::cmp::Ordering;

use super::terms::Stats;
use super::token;
use crate::store::Observation;

/// BM25 term-frequency saturation.
const K1: f64 = 1.2;
/// BM25 length normalisation.
const B: f64 = 0.75;

/// A row and its score.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Scored {
    pub(super) score: f64,
    pub(super) row: Observation,
}

/// What a query keeps: its terms under global stats, its row filters
/// past the store's own, and how many best rows each source keeps.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Rater {
    pub(super) stats: Stats,
    /// Distinct and sorted ([`token::terms`]), aligned with `stats.df`.
    pub(super) terms: Vec<String>,
    pub(super) kind: Option<String>,
    pub(super) session: Option<String>,
    pub(super) keep: usize,
}

impl Rater {
    /// `row` scored, or `None` when a filter refuses it or no term
    /// occurs in it.
    pub(super) fn rate(&self, row: Observation) -> Option<Scored> {
        if !self.admits(&row) {
            return None;
        }
        let score = score(&self.stats, &self.terms, &row.body);
        (score > 0.0).then_some(Scored { score, row })
    }

    /// Whether the row filters past the store's own take `row`.
    fn admits(&self, row: &Observation) -> bool {
        let wants = |want: &Option<String>, have: &str| {
            want.as_deref().is_none_or(|w| w == have)
        };
        wants(&self.kind, &row.kind) && wants(&self.session, &row.session)
    }

    /// The best [`Rater::keep`] of `rows`, best first.
    pub(super) fn best(&self, rows: Vec<Observation>) -> Vec<Scored> {
        top(
            rows.into_iter().filter_map(|r| self.rate(r)).collect(),
            self.keep,
        )
    }

    /// [`Rater::best`] of lent rows an index counted: scored from their
    /// [`Counts`] by [`score`]'s arithmetic (V1), only the kept cloned.
    pub(super) fn best_counted<'a>(
        &self,
        rows: impl IntoIterator<Item = (&'a Observation, Counts)>,
    ) -> Vec<Scored> {
        let mut rated: Vec<(f64, &Observation)> = rows
            .into_iter()
            .filter(|(row, _)| self.admits(row))
            .map(|(row, c)| (counted(&self.stats, &c), row))
            .filter(|(score, _)| *score > 0.0)
            .collect();
        rated.sort_by(|a, b| order_of(*a, *b));
        rated.truncate(self.keep);
        let kept = rated.into_iter().map(|(score, row)| Scored {
            score,
            row: row.clone(),
        });
        kept.collect()
    }
}

/// A row's counts as an index keeps them: its token count and, per
/// query term in order, how often that term occurs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Counts {
    pub(super) dl: u32,
    pub(super) tf: Vec<u32>,
}

/// BM25 of `body` for `terms` under `stats`, summed in term order, so
/// the same row gets the same bits every time (V1).
pub(super) fn score(stats: &Stats, terms: &[String], body: &str) -> f64 {
    let tokens = token::tokens(body);
    let tf = terms
        .iter()
        .map(|term| count(tokens.iter().filter(|t| *t == term).count()));
    bm25(stats, count(tokens.len()), tf)
}

/// [`score`] from a row's [`Counts`]: the same terms in the same order
/// give the same bits (V1).
fn counted(stats: &Stats, c: &Counts) -> f64 {
    let tf = c.tf.iter().map(|&n| real(u64::from(n)));
    bm25(stats, real(u64::from(c.dl)), tf)
}

/// BM25 of a row of `dl` tokens, each term's `tf` aligned with
/// `stats.df`, summed in term order.
fn bm25(stats: &Stats, dl: f64, tf: impl Iterator<Item = f64>) -> f64 {
    let norm = length_norm(stats, dl);
    let each = tf.zip(&stats.df).map(|(tf, &df)| {
        idf(stats.docs, df) * tf * (K1 + 1.0) / K1.mul_add(norm, tf)
    });
    each.sum()
}

/// `1 - b + b * dl / avgdl`; 1 before any row is counted.
fn length_norm(stats: &Stats, dl: f64) -> f64 {
    if stats.docs == 0 || stats.tokens == 0 {
        return 1.0;
    }
    let avg = real(stats.tokens) / real(stats.docs);
    B.mul_add(dl / avg, 1.0 - B)
}

/// `ln(1 + (N - df + 0.5) / (df + 0.5))`: positive for every df, so a
/// term in every row still counts.
fn idf(docs: u64, df: u64) -> f64 {
    let (n, df) = (real(docs), real(df));
    (((n - df).max(0.0) + 0.5) / (df + 0.5)).ln_1p()
}

/// Best first: score desc, then ts desc, then id desc, so the order is
/// total and the same on every run (V4).
pub(super) fn order(a: &Scored, b: &Scored) -> Ordering {
    order_of((a.score, &a.row), (b.score, &b.row))
}

/// [`order`] of a score and a lent row each.
fn order_of(a: (f64, &Observation), b: (f64, &Observation)) -> Ordering {
    let ((sa, ra), (sb, rb)) = (a, b);
    sb.total_cmp(&sa)
        .then(rb.ts.cmp(&ra.ts))
        .then_with(|| rb.id.cmp(&ra.id))
}

/// The best `k` of `rows`, best first.
pub(super) fn top(mut rows: Vec<Scored>, k: usize) -> Vec<Scored> {
    rows.sort_by(order);
    rows.truncate(k);
    rows
}

fn count(n: usize) -> f64 {
    real(u64::try_from(n).unwrap_or(u64::MAX))
}

// Row and token counts stay far below 2^52, where f64 turns lossy.
#[allow(clippy::cast_precision_loss)]
fn real(n: u64) -> f64 {
    n as f64
}
