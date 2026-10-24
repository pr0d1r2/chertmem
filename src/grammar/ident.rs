//! The identifier rule of cavemem's `tokenize.ts`, tried at one byte
//! index: `\b[A-Za-z_][A-Za-z0-9_]*[-_][A-Za-z0-9_\-]+\b` (snake and
//! kebab case), else `\b[a-z]+[A-Z][A-Za-z0-9]*\b` (camel case).

#[cfg(test)]
mod tests;

use super::scan::{at, boundary, is_word, run};

fn joiner(c: u8) -> bool {
    matches!(c, b'-' | b'_')
}

fn tail_part(c: u8) -> bool {
    is_word(c) || c == b'-'
}

/// `[A-Za-z0-9_\-]+\b` from `q`: the longest run ending at a `\b`.
fn tail(b: &[u8], q: usize) -> Option<usize> {
    let end = run(b, q, tail_part);
    (q.saturating_add(1)..=end).rev().find(|&e| boundary(b, e))
}

/// The snake or kebab alternative, its `*` backtracking from longest.
fn joined(b: &[u8], p: usize) -> Option<usize> {
    at(b, p).filter(|&c| c.is_ascii_alphabetic() || c == b'_')?;
    let head = p.checked_add(1)?;
    let most = run(b, head, is_word);
    (head..=most).rev().find_map(|j| {
        at(b, j).filter(|&c| joiner(c))?;
        tail(b, j.checked_add(1)?)
    })
}

/// The camel case alternative.
fn camel(b: &[u8], p: usize) -> Option<usize> {
    let lower = run(b, p, |c| c.is_ascii_lowercase());
    (lower > p).then_some(())?;
    at(b, lower).filter(u8::is_ascii_uppercase)?;
    let end = run(b, lower.checked_add(1)?, |c| c.is_ascii_alphanumeric());
    boundary(b, end).then_some(end)
}

/// The identifier rule at `p`.
pub(super) fn identifier(s: &str, p: usize) -> Option<usize> {
    let b = s.as_bytes();
    boundary(b, p).then_some(())?;
    joined(b, p).or_else(|| camel(b, p))
}
