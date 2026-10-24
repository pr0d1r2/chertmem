//! cavemem's `tokenize.ts`: every rule scans the whole text as a global
//! regex, the spans are sorted by start, then priority, then length,
//! overlaps are dropped first come, and the gaps are prose. Two changes:
//! inline code is scanned again with each kept fence's end as a cut, so
//! the fence's last tick opens no inline code (src/grammar V5 (b)), and
//! an article word that starts a path or file name is a path (V5 (d)).

#[cfg(test)]
mod tests;

use std::cmp::Reverse;

use super::ident::identifier;
use super::numeric::{date, number, version};
use super::rules::{article_path, fence, heading, inline, path, url};

type Rule = fn(&str, usize) -> Option<usize>;

/// The rules with their priorities, as in `RULES`, and chert's article
/// path as a path (V5 (d)): it starts where no other rule's span does.
const RULES: [(u8, Rule); 10] = [
    (100, fence),
    (90, inline),
    (80, url),
    (70, heading),
    (60, path),
    (60, article_path),
    (50, date),
    (40, version),
    (30, number),
    (20, identifier),
];

/// The lowest priority of a code rule: fence and inline code; it is
/// inline code's own.
const CODE: u8 = 90;

/// The priority of the fence rule.
const FENCE: u8 = 100;

/// The priority of the path rule.
const PATH: u8 = 60;

/// One piece of the text: kept as it is, or prose to rewrite. `code`
/// marks a kept fence or inline code span (src/grammar V4), `path` a
/// kept path (V5 (c)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Segment<'a> {
    pub(super) text: &'a str,
    pub(super) kept: bool,
    pub(super) code: bool,
    pub(super) path: bool,
}

#[derive(Debug, Clone, Copy)]
struct Span {
    start: usize,
    end: usize,
    priority: u8,
}

/// Every match of one rule, as `RegExp.exec` with `g` finds them, but a
/// match across a cut is dropped and the scan goes on from the cut.
fn scan(s: &str, (priority, rule): (u8, Rule), cuts: &[usize]) -> Vec<Span> {
    let mut out = Vec::new();
    let mut p = 0;
    while p < s.len() {
        let found = rule(s, p).filter(|&end| end > p);
        let cut = found.and_then(|end| crossing(cuts, p, end));
        let span = found.map(|end| Span {
            start: p,
            end,
            priority,
        });
        out.extend(span.filter(|_| cut.is_none()));
        p = cut.or(found).unwrap_or_else(|| p.saturating_add(1));
    }
    out
}

/// The first cut strictly inside `start..end`.
fn crossing(cuts: &[usize], start: usize, end: usize) -> Option<usize> {
    cuts.iter().copied().find(|&c| start < c && c < end)
}

/// Every rule's matches; inline code is cut at `cuts`.
fn candidates(s: &str, cuts: &[usize]) -> Vec<Span> {
    let cut_for = |priority| if priority == CODE { cuts } else { &[] };
    RULES
        .into_iter()
        .flat_map(|(priority, rule)| {
            scan(s, (priority, rule), cut_for(priority))
        })
        .collect()
}

/// The spans that survive overlap resolution, in text order.
fn pick(mut spans: Vec<Span>) -> Vec<Span> {
    spans.sort_by_key(|x| (x.start, Reverse(x.priority), Reverse(x.end)));
    let mut cursor = 0;
    spans.retain(|x| {
        let keep = x.start >= cursor;
        cursor = if keep { x.end } else { cursor };
        keep
    });
    spans
}

/// cavemem's spans, then again with inline code cut at the end of each
/// fence they keep (V5 (b)): with no fence kept, cavemem's spans.
fn resolve(s: &str) -> Vec<Span> {
    let first = pick(candidates(s, &[]));
    let ends: Vec<usize> = first
        .iter()
        .filter(|x| x.priority == FENCE)
        .map(|x| x.end)
        .collect();
    if ends.is_empty() {
        return first;
    }
    pick(candidates(s, &ends))
}

/// One segment of `text`: prose, or a kept span of rule `priority`.
fn segment(text: &str, priority: Option<u8>) -> Segment<'_> {
    Segment {
        text,
        kept: priority.is_some(),
        code: priority >= Some(CODE),
        path: priority == Some(PATH),
    }
}

/// `text` cut into kept spans and the prose between them.
pub(super) fn tokenize(text: &str) -> Vec<Segment<'_>> {
    let mut out = Vec::new();
    let mut pos = 0;
    let mut push = |from: usize, to: usize, priority: Option<u8>| {
        let piece = text.get(from..to).filter(|t| !t.is_empty());
        out.extend(piece.map(|t| segment(t, priority)));
    };
    for span in resolve(text) {
        push(pos, span.start, None);
        push(span.start, span.end, Some(span.priority));
        pos = span.end;
    }
    push(pos, text.len(), None);
    out
}
