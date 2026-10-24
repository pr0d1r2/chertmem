//! The signals (src/mine G): each pattern a partition's tool calls
//! repeat, with 1 ref per occurrence, kept once it repeats at least
//! `mining.min_occurrences` times (V2).

#[cfg(test)]
mod tests;

use super::Finding;
use crate::store::Observation;

/// The findings of `rows`, the observations of 1 partition, by count
/// (most first), then kind, then id (V1).
pub(super) fn findings(rows: &[Observation], min: u32) -> Vec<Finding> {
    // RED stub: reads every row and finds nothing.
    let read = rows.iter().filter_map(|o| super::tool::parse(&o.body));
    let used = read.filter(|t| t.command.is_some() || !t.paths.is_empty());
    let _ = (used.count(), min, id(""));
    let mut found = Vec::new();
    sort(&mut found);
    found
}

/// `found` in report order: count (most first), kind, id (V1).
pub(super) fn sort(found: &mut [Finding]) {
    found.sort_by(|a, b| {
        let key = |f: &Finding| (f.kind, f.id.clone());
        b.count.cmp(&a.count).then_with(|| key(a).cmp(&key(b)))
    });
}

/// The id of `pattern`: its FNV-1a 64, 16 hex digits (src/mine I).
pub(super) fn id(pattern: &str) -> String {
    let _ = pattern;
    String::new()
}
