//! `perf-doc` (dev T9, docs V1): `docs/PERFORMANCE.md` rendered from
//! `benches/baseline.toml` and the latency targets of the root
//! `SPEC.md` C, the same bytes for the same inputs.

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use crate::baseline::Row;

/// The root C latency targets, by name, in ms.
///
/// # Errors
/// No such line, or an item not of that shape.
pub fn targets(spec: &str) -> Result<BTreeMap<String, f64>, String> {
    let _ = spec;
    Ok(BTreeMap::new())
}

/// The page for `rows`, 1 section per suite in the rows' order.
///
/// # Errors
/// A row naming a target `targets` does not hold.
pub fn render(
    rows: &[Row],
    targets: &BTreeMap<String, f64>,
) -> Result<String, String> {
    let _ = (rows, targets);
    Ok(String::new())
}
