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
    denylist
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_lowercase)
        .collect()
}

/// Every line of every file that holds a name from `names`.
#[must_use]
pub fn scan(names: &[String], files: &[(String, String)]) -> Vec<Finding> {
    files
        .iter()
        .flat_map(|(path, text)| hits(names, path, text))
        .collect()
}

fn hits(names: &[String], path: &str, text: &str) -> Vec<Finding> {
    text.lines()
        .zip(1..)
        .filter(|(l, _)| {
            let low = l.to_lowercase();
            names.iter().any(|n| low.contains(n.as_str()))
        })
        .map(|(_, line)| Finding {
            path: path.to_owned(),
            line,
        })
        .collect()
}
