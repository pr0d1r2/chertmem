//! The protected-span rules of cavemem's `tokenize.ts` that are not
//! numbers or identifiers: fence, inline code, URL, heading, path, and
//! chert's own article path (src/grammar V5 (d)). Each is the regex
//! tried at one byte index, giving the match end.

#[cfg(test)]
mod tests;

use super::lexicon::ARTICLES;
use super::scan::{at, boundary, char_at, eat, js_space, line_start, run};

/// ```` ```[\s\S]*?```|~~~[\s\S]*?~~~ ````: the shortest closed fence.
pub(super) fn fence(s: &str, p: usize) -> Option<usize> {
    ["```", "~~~"].into_iter().find_map(|mark| {
        let rest = s.get(p..)?.strip_prefix(mark)?;
        let close = rest.find(mark)?;
        p.checked_add(close)?
            .checked_add(mark.len().checked_mul(2)?)
    })
}

/// `` `[^`\n]+` ``: inline code on one line.
pub(super) fn inline(s: &str, p: usize) -> Option<usize> {
    let b = s.as_bytes();
    let open = eat(b, p, b'`')?;
    let end = run(b, open, |c| c != b'`' && c != b'\n');
    (end > open).then_some(())?;
    eat(b, end, b'`')
}

/// `\bhttps?:\/\/[^\s)\]]+`: a URL up to white space, `)` or `]`.
pub(super) fn url(s: &str, p: usize) -> Option<usize> {
    boundary(s.as_bytes(), p).then_some(())?;
    let rest = s.get(p..)?;
    let tail = rest
        .strip_prefix("https://")
        .or_else(|| rest.strip_prefix("http://"))?;
    let len = tail
        .find(|c| js_space(c) || c == ')' || c == ']')
        .unwrap_or(tail.len());
    (len > 0).then_some(())?;
    s.len().checked_sub(tail.len())?.checked_add(len)
}

/// `^#{1,6}\s[^\n]*$` under `m`: a markdown heading line.
pub(super) fn heading(s: &str, p: usize) -> Option<usize> {
    line_start(s, p).then_some(())?;
    let marks = run(s.as_bytes(), p, |c| c == b'#');
    (1..=6).contains(&marks.saturating_sub(p)).then_some(())?;
    let gap = char_at(s, marks).filter(|&c| js_space(c))?;
    let body = marks.checked_add(gap.len_utf8())?;
    let len = s.get(body..)?.find('\n');
    Some(len.map_or(s.len(), |n| body.saturating_add(n)))
}

fn unix_part(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-' | b'/')
}

fn windows_part(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-' | b'\\')
}

/// One or more `keep` bytes from `i`, giving their end.
fn some(b: &[u8], i: usize, keep: fn(u8) -> bool) -> Option<usize> {
    let end = run(b, i, keep);
    (end > i).then_some(end)
}

/// `(?:\.{1,2})?\/[A-Za-z0-9._\-/]+`: relative or absolute unix path.
fn unix(b: &[u8], p: usize) -> Option<usize> {
    let dots = run(b, p, |c| c == b'.');
    (dots.saturating_sub(p) <= 2).then_some(())?;
    some(b, eat(b, dots, b'/')?, unix_part)
}

/// `~\/[A-Za-z0-9._\-/]+`: a path under home.
fn home(b: &[u8], p: usize) -> Option<usize> {
    some(b, eat(b, eat(b, p, b'~')?, b'/')?, unix_part)
}

/// `[A-Z]:\\[A-Za-z0-9._\-\\]+`: a Windows path.
fn windows(b: &[u8], p: usize) -> Option<usize> {
    at(b, p).filter(u8::is_ascii_uppercase)?;
    let colon = eat(b, p.checked_add(1)?, b':')?;
    some(b, eat(b, colon, b'\\')?, windows_part)
}

/// The path rule: its three alternatives in order.
pub(super) fn path(s: &str, p: usize) -> Option<usize> {
    let b = s.as_bytes();
    unix(b, p).or_else(|| home(b, p)).or_else(|| windows(b, p))
}

/// `/` then a path byte, or `.` then a letter or digit, at `q`.
fn starts_name(b: &[u8], q: usize) -> bool {
    let next = q.checked_add(1).and_then(|i| at(b, i));
    match at(b, q) {
        Some(b'/') => next.is_some_and(unix_part),
        Some(b'.') => next.is_some_and(|c| c.is_ascii_alphanumeric()),
        _ => false,
    }
}

/// chert's own rule (src/grammar V5 (d)): an article word at a `\b`, any
/// case, that starts a path or file name (`a/b.ipynb`, `the.md`), with
/// the whole `[A-Za-z0-9._\-/]` run after it. cavemem has no such rule,
/// so its article filter splits the word off.
pub(super) fn article_path(s: &str, p: usize) -> Option<usize> {
    let b = s.as_bytes();
    boundary(b, p).then_some(())?;
    ARTICLES.iter().find_map(|word| {
        let q = p.checked_add(word.len())?;
        let found = b.get(p..q)?.eq_ignore_ascii_case(word.as_bytes());
        (found && starts_name(b, q)).then(|| run(b, q, unix_part))
    })
}
