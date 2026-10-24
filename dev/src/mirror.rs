//! `rust-mirror`: every Rust module with logic has its unit tests beside
//! it, wired in, and every `tests.rs` has its module (dev V7, a port of
//! the xenolith v0.1.0 rule lablith copies as shell).

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

/// Names of every `fn` defined with a body, `//` comments dropped.
#[must_use]
pub fn bodied_fns(text: &str) -> Vec<String> {
    let _ = text;
    Vec::new()
}

/// Whether the module declares `#[cfg(test)] mod tests;`.
#[must_use]
pub fn wired(text: &str) -> bool {
    let _ = text;
    true
}

/// Every broken mirror among the tracked `.rs` files (path to text),
/// given the crate roots (`""` for the repository root, `"dev/"` ...).
#[must_use]
pub fn findings(
    rs: &BTreeMap<String, String>,
    crates: &[String],
) -> Vec<String> {
    let _ = (rs, crates);
    Vec::new()
}
