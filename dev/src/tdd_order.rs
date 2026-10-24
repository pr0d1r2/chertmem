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
    let mut ids = BTreeSet::new();
    for (raw, n) in text.lines().zip(1..) {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (id, reason) = line.split_once(' ').unwrap_or((line, ""));
        entry(id, reason, n)?;
        ids.insert(id.to_owned());
    }
    Ok(ids)
}

fn entry(id: &str, reason: &str, n: usize) -> Result<(), String> {
    let full = id.len() == 40
        && id.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'));
    if !full {
        return Err(format!(
            "line {n}: an exempt commit is named by its full 40-character id"
        ));
    }
    if reason.trim().is_empty() {
        return Err(format!("line {n}: {id} has no reason"));
    }
    Ok(())
}

/// Every commit that adds a `.rs` file outside `tests/` before any test
/// commit in the range, unless exempt.
#[must_use]
pub fn findings(commits: &[Commit], exempt: &BTreeSet<String>) -> Vec<String> {
    let mut seen_test = false;
    let mut out = Vec::new();
    for c in commits.iter().filter(|c| !exempt.contains(&c.id)) {
        if is_test(&c.subject) {
            seen_test = true;
        } else if !seen_test {
            out.extend(
                c.added.iter().filter(|f| is_code(f)).map(|f| finding(c, f)),
            );
        }
    }
    out
}

/// A `test:` commit is the RED side: whatever it adds is evidence.
fn is_test(subject: &str) -> bool {
    subject.starts_with("test:") || subject.starts_with("test(")
}

/// Rust code outside the `tests/` trees, which are tests and fixtures.
fn is_code(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
        && !path.starts_with("tests/")
        && !path.contains("/tests/")
}

fn finding(c: &Commit, file: &str) -> String {
    let short = c.id.get(..7).unwrap_or(&c.id);
    format!(
        "{short} adds {file} with no `test:` commit before it in the range. Rust tests have no mirrored path, so TDD order is checked on the commit type: commit the failing test first (dev V8)."
    )
}
