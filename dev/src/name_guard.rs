//! `name-guard`: no committed file holds a name from the local, untracked
//! `.private/denylist` (dev V3, `paradigm/V37`).

#[cfg(test)]
mod tests;

/// One file that holds a denylisted name, by path and 1-based line. The
/// name itself is never carried, so a report cannot leak it.
#[derive(Debug, PartialEq, Eq)]
pub struct Finding {
    pub path: String,
    pub line: usize,
}

/// The names in a denylist: one per line, blank lines and `#` comments
/// skipped, lowercased for a case-insensitive match.
#[must_use]
pub fn names(denylist: &str) -> Vec<String> {
    let _ = denylist;
    Vec::new()
}

/// Every line of every file that holds a name from `names`.
#[must_use]
pub fn scan(names: &[String], files: &[(String, String)]) -> Vec<Finding> {
    let _ = (names, files);
    Vec::new()
}
