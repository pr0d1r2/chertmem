//! cavemem's `tokenize.ts`: every rule scans the whole text as a global
//! regex, the spans are sorted by start, then priority, then length,
//! overlaps are dropped first come, and the gaps are prose.

#[cfg(test)]
mod tests;

use std::cmp::Reverse;

use super::ident::identifier;
use super::numeric::{date, number, version};
use super::rules::{fence, heading, inline, path, url};

type Rule = fn(&str, usize) -> Option<usize>;

/// The rules with their priorities, as in `RULES`.
const RULES: [(u8, Rule); 9] = [
    (100, fence),
    (90, inline),
    (80, url),
    (70, heading),
    (60, path),
    (50, date),
    (40, version),
    (30, number),
    (20, identifier),
];

/// The lowest priority of a code rule: fence and inline code.
const CODE: u8 = 90;

/// One piece of the text: kept as it is, or prose to rewrite. `code`
/// marks a kept fence or inline code span (src/grammar V4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Segment<'a> {
    pub(super) text: &'a str,
    pub(super) kept: bool,
    pub(super) code: bool,
}

#[derive(Debug, Clone, Copy)]
struct Span {
    start: usize,
    end: usize,
    priority: u8,
}

/// Every match of one rule, as `RegExp.exec` with `g` finds them.
fn scan(s: &str, priority: u8, rule: Rule, out: &mut Vec<Span>) {
    let mut p = 0;
    while p < s.len() {
        match rule(s, p).filter(|&end| end > p) {
            Some(end) => {
                out.push(Span {
                    start: p,
                    end,
                    priority,
                });
                p = end;
            }
            None => p = p.saturating_add(1),
        }
    }
}

/// The spans that survive overlap resolution, in text order.
fn resolve(s: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    for (priority, rule) in RULES {
        scan(s, priority, rule, &mut spans);
    }
    spans.sort_by_key(|x| (x.start, Reverse(x.priority), Reverse(x.end)));
    let mut cursor = 0;
    spans.retain(|x| {
        let keep = x.start >= cursor;
        cursor = if keep { x.end } else { cursor };
        keep
    });
    spans
}

/// `text` cut into kept spans and the prose between them.
pub(super) fn tokenize(text: &str) -> Vec<Segment<'_>> {
    let mut out = Vec::new();
    let mut pos = 0;
    let mut push = |from: usize, to: usize, kept: bool, code: bool| {
        let piece = text.get(from..to).filter(|t| !t.is_empty());
        out.extend(piece.map(|text| Segment { text, kept, code }));
    };
    for span in resolve(text) {
        push(pos, span.start, false, false);
        push(span.start, span.end, true, span.priority >= CODE);
        pos = span.end;
    }
    push(pos, text.len(), false, false);
    out
}
