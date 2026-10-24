//! `text.replace(/\b(?:k1|k2|...)\b/gi, f)` for literal ASCII keys, as
//! cavemem builds it in `removePhrases`, `abbreviate` and `expand`, and
//! the `matchCase` that carries the found word's case to its target.

#[cfg(test)]
mod tests;

use super::scan::boundary;

/// The first key, in order, found at `p` ignoring ASCII case and ending
/// at a `\b`, as its index and the end of the match.
fn key_at(b: &[u8], p: usize, keys: &[&str]) -> Option<(usize, usize)> {
    boundary(b, p).then_some(())?;
    keys.iter().enumerate().find_map(|(k, key)| {
        let end = p.checked_add(key.len())?;
        let found = b.get(p..end)?.eq_ignore_ascii_case(key.as_bytes());
        (found && boundary(b, end)).then_some((k, end))
    })
}

/// Every match of the alternation over `keys` in `text`, left to right
/// and never overlapping, replaced by `with(found, key index)`.
pub(super) fn replace(
    text: &str,
    keys: &[&str],
    with: impl Fn(&str, usize) -> String,
) -> String {
    let b = text.as_bytes();
    let (mut out, mut copied, mut p) = (String::new(), 0, 0);
    while p < b.len() {
        let Some((k, end)) = key_at(b, p, keys) else {
            p = p.saturating_add(1);
            continue;
        };
        out.push_str(text.get(copied..p).unwrap_or_default());
        out.push_str(&with(text.get(p..end).unwrap_or_default(), k));
        (copied, p) = (end, end);
    }
    out.push_str(text.get(copied..).unwrap_or_default());
    out
}

/// `matchCase`: all caps stays all caps, a capital first letter stays.
pub(super) fn match_case(source: &str, target: &str) -> String {
    if source == source.to_uppercase() {
        return target.to_uppercase();
    }
    let first = source.chars().next();
    if first.is_some_and(|c| c.to_uppercase().eq([c])) {
        let mut chars = target.chars();
        let head: String = chars
            .next()
            .into_iter()
            .flat_map(char::to_uppercase)
            .collect();
        return format!("{head}{}", chars.as_str());
    }
    target.to_owned()
}
