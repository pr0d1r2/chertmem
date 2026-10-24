//! The date, version and number rules of cavemem's `tokenize.ts`, each
//! the regex tried at one byte index with its backtracking order kept:
//! optional groups are tried taken first, and the first end at a `\b`
//! wins.

#[cfg(test)]
mod tests;

use super::scan::{
    at, boundary, char_at, digits, eat, is_word, run, some_digits,
};

/// The first candidate end at a `\b`.
fn first_at_boundary(b: &[u8], ends: &[Option<usize>]) -> Option<usize> {
    ends.iter().flatten().copied().find(|&e| boundary(b, e))
}

/// `[T\s]\d{2}:\d{2}`: the time after a date ending at `d`.
fn clock(s: &str, d: usize) -> Option<usize> {
    let gap =
        char_at(s, d).filter(|&c| c == 'T' || super::scan::js_space(c))?;
    let b = s.as_bytes();
    let hours = digits(b, d.checked_add(gap.len_utf8())?, 2)?;
    digits(b, eat(b, hours, b':')?, 2)
}

/// `\b\d{4}-\d{2}-\d{2}(?:[T\s]\d{2}:\d{2}(?::\d{2})?)?\b`.
pub(super) fn date(s: &str, p: usize) -> Option<usize> {
    let b = s.as_bytes();
    boundary(b, p).then_some(())?;
    let year = digits(b, p, 4)?;
    let month = digits(b, eat(b, year, b'-')?, 2)?;
    let day = digits(b, eat(b, month, b'-')?, 2)?;
    let minutes = clock(s, day);
    let seconds = minutes.and_then(|m| digits(b, eat(b, m, b':')?, 2));
    first_at_boundary(b, &[seconds, minutes, Some(day)])
}

/// `(?:[-+][\w.]+)?\b` from `x`: the longest suffix ending at a `\b`,
/// else `x` itself if a `\b` is there.
fn suffix(b: &[u8], x: usize) -> Option<usize> {
    let tagged = at(b, x).filter(|c| matches!(c, b'-' | b'+'));
    let tag = tagged.and_then(|_| x.checked_add(1)).and_then(|start| {
        let end = run(b, start, |c| is_word(c) || c == b'.');
        (start.saturating_add(1)..=end)
            .rev()
            .find(|&e| boundary(b, e))
    });
    tag.or_else(|| boundary(b, x).then_some(x))
}

/// `\bv?\d+\.\d+(?:\.\d+)?(?:[-+][\w.]+)?\b`.
pub(super) fn version(s: &str, p: usize) -> Option<usize> {
    let b = s.as_bytes();
    boundary(b, p).then_some(())?;
    let start = eat(b, p, b'v').unwrap_or(p);
    let major = some_digits(b, start)?;
    let minor = some_digits(b, eat(b, major, b'.')?)?;
    let patch = eat(b, minor, b'.').and_then(|d| some_digits(b, d));
    patch
        .and_then(|x| suffix(b, x))
        .or_else(|| suffix(b, minor))
}

/// `\b\d+(?:\.\d+)?\b`.
pub(super) fn number(s: &str, p: usize) -> Option<usize> {
    let b = s.as_bytes();
    boundary(b, p).then_some(())?;
    let whole = some_digits(b, p)?;
    let fraction = eat(b, whole, b'.').and_then(|d| some_digits(b, d));
    first_at_boundary(b, &[fraction, Some(whole)])
}
