//! The optional vector re-rank (src/search T4, D18): the keyword best
//! are embedded with the query and re-scored as
//! `(1 - alpha) * bm25 / best_bm25 + alpha * cosine`. Brute-force
//! cosine until a node measures need for ANN (src/search C).

#[cfg(test)]
mod tests;

use super::Error;
use super::rank::{self, Scored};

/// Turns texts into vectors, 1 per text, in order.
pub(super) trait Embed {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f64>>, Error>;
}

/// `rows` (best first) re-scored by `alpha` and re-ordered (V4).
pub(super) fn rerank(
    embed: &impl Embed,
    query: &str,
    rows: Vec<Scored>,
    alpha: f64,
) -> Result<Vec<Scored>, Error> {
    let Some(best) = rows.first().map(|s| s.score) else {
        return Ok(rows);
    };
    let (q, each) = embedded(embed, query, &rows)?;
    let keep = rows.len();
    let blended = rows.into_iter().zip(&each).map(|(s, v)| Scored {
        score: blend(s.score / best, cosine(&q, v), alpha),
        row: s.row,
    });
    Ok(rank::top(blended.collect(), keep))
}

/// The query's vector and 1 per row, in order.
fn embedded(
    embed: &impl Embed,
    query: &str,
    rows: &[Scored],
) -> Result<(Vec<f64>, Vec<Vec<f64>>), Error> {
    let mut texts = vec![query];
    texts.extend(rows.iter().map(|s| s.row.body.as_str()));
    let mut vectors = embed.embed(&texts)?.into_iter();
    let q = vectors.next().unwrap_or_default();
    let each: Vec<Vec<f64>> = vectors.collect();
    if each.len() != rows.len() {
        return Err(Error::Embed {
            reason: "not 1 vector per text".into(),
        });
    }
    Ok((q, each))
}

/// `(1 - alpha) * keyword + alpha * vector`.
pub(super) fn blend(keyword: f64, vector: f64, alpha: f64) -> f64 {
    (1.0 - alpha).mul_add(keyword, alpha * vector)
}

/// Cosine similarity; 0 for a zero vector or differing lengths.
pub(super) fn cosine(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() {
        return 0.0;
    }
    let dot = |x: &[f64], y: &[f64]| -> f64 {
        x.iter().zip(y).fold(0.0, |acc, (p, q)| p.mul_add(*q, acc))
    };
    let norm = (dot(a, a) * dot(b, b)).sqrt();
    if norm > 0.0 { dot(a, b) / norm } else { 0.0 }
}
