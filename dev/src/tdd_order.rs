//! `tdd-order`: every commit that adds Rust code has a `test:` commit
//! before it in the range, unless exempt (dev V8, a port of the xenolith
//! v0.1.0 rule lablith copies as shell, Rust rule only).

#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

/// One commit of the range: its full id, subject and the paths it adds.
#[derive(Debug, Clone)]
pub struct Commit {
    pub id: String,
    pub subject: String,
    pub added: Vec<String>,
}

/// The exempt commit ids of a `tdd-order.exempt` text: one full 40-hex id
/// and a reason per line, blank lines and `#` comments skipped. An entry
/// that is malformed is an error naming its line.
///
/// # Errors
/// A line whose id is not 40 lowercase hex characters, or has no reason.
pub fn exempt(text: &str) -> Result<BTreeSet<String>, String> {
    let _ = text;
    Ok(BTreeSet::new())
}

/// Every commit that adds a `.rs` file outside `tests/` before any test
/// commit in the range, unless exempt.
#[must_use]
pub fn findings(commits: &[Commit], exempt: &BTreeSet<String>) -> Vec<String> {
    let _ = (commits, exempt);
    Vec::new()
}
