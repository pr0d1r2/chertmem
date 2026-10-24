//! cavemem's `compressProse` and `collapseWhitespace`: phrases removed,
//! words abbreviated, white space collapsed, the edges padded back.

#[cfg(test)]
mod tests;

use super::lexicon::Grammar;
use super::scan::{line_end, space_prefix, space_suffix};
use super::words::{match_case, replace};

/// One global regex replace, tried at the start of `rest` with the text
/// `before` it: the bytes it consumes and what it writes instead.
type Step = for<'a> fn(&str, &'a str) -> Option<(usize, &'a str)>;

/// `text.replace(re, ...)` for the regex `step` stands for.
fn rewrite(text: &str, step: Step) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    while let Some(rest) = text.get(pos..).filter(|r| !r.is_empty()) {
        let before = text.get(..pos).unwrap_or_default();
        let one = rest.chars().next().map_or(1, char::len_utf8);
        let copy = (one, rest.get(..one).unwrap_or_default());
        let (used, emit) = step(before, rest).unwrap_or(copy);
        out.push_str(emit);
        pos = pos.saturating_add(used);
    }
    out
}

/// The length in bytes of the run of `keep` chars that starts `s`.
fn lead(s: &str, keep: impl Fn(char) -> bool) -> usize {
    s.len().saturating_sub(s.trim_start_matches(keep).len())
}

/// `/[ \t]+/g` to `" "`.
fn blanks<'a>(_: &str, rest: &'a str) -> Option<(usize, &'a str)> {
    let n = lead(rest, |c| c == ' ' || c == '\t');
    (n > 0).then_some((n, " "))
}

/// `/ ?\n ?/g` to `"\n"`.
fn newline<'a>(_: &str, rest: &'a str) -> Option<(usize, &'a str)> {
    let space = usize::from(rest.starts_with(' '));
    let after = rest.get(space..)?.strip_prefix('\n')?;
    let tail = usize::from(after.starts_with(' '));
    Some((space.saturating_add(1).saturating_add(tail), "\n"))
}

/// A run of `blank` chars, then one of `.,;:!?`: the mark alone.
fn mark_after(rest: &str, blank: fn(char) -> bool) -> Option<(usize, &str)> {
    let n = lead(rest, blank);
    (n > 0).then_some(())?;
    let end = n.checked_add(1)?;
    let mark = rest.get(n..end).filter(|m| ".,;:!?".contains(*m))?;
    Some((end, mark))
}

/// `/ +([.,;:!?])/g` to `"$1"`.
fn space_marks<'a>(_: &str, rest: &'a str) -> Option<(usize, &'a str)> {
    mark_after(rest, |c| c == ' ')
}

/// `/[ \t]+([.,;:!?])/g` to `"$1"`, as `compress` ends.
fn blank_marks<'a>(_: &str, rest: &'a str) -> Option<(usize, &'a str)> {
    mark_after(rest, |c| c == ' ' || c == '\t')
}

/// `/\n{3,}/g` to `"\n\n"`.
fn blank_lines<'a>(_: &str, rest: &'a str) -> Option<(usize, &'a str)> {
    let n = lead(rest, |c| c == '\n');
    (n >= 3).then_some((n, "\n\n"))
}

/// `/^ +| +$/gm` to `""`.
fn edge_spaces<'a>(before: &str, rest: &'a str) -> Option<(usize, &'a str)> {
    let n = lead(rest, |c| c == ' ');
    (n > 0).then_some(())?;
    let start = before.chars().next_back().is_none_or(line_end);
    let end = rest.get(n..)?.chars().next().is_none_or(line_end);
    (start || end).then_some((n, ""))
}

/// `collapseWhitespace`.
fn collapse(text: &str) -> String {
    let steps: [Step; 5] =
        [blanks, newline, space_marks, blank_lines, edge_spaces];
    steps
        .into_iter()
        .fold(text.to_owned(), |t, step| rewrite(&t, step))
}

/// The final pass of `compress` over the joined segments.
pub(super) fn tighten(text: &str) -> String {
    rewrite(text, blank_marks)
}

/// `removePhrases` for each list, then `abbreviate`.
fn rewrite_words(body: &str, grammar: &Grammar) -> String {
    let removed = grammar.removals.iter().fold(body.to_owned(), |t, keys| {
        replace(&t, keys, |_, _| String::new())
    });
    let pairs = grammar.abbreviations.iter().copied().flatten();
    pairs.fold(removed, |t, (from, to)| {
        replace(&t, &[from], |found, _| match_case(found, to))
    })
}

/// The pad an edge of white space leaves: a newline, a space or none.
fn pad(edge: &str) -> &'static str {
    match (edge.contains('\n'), edge.is_empty()) {
        (true, _) => "\n",
        (false, true) => "",
        (false, false) => " ",
    }
}

/// `compressProse`: one prose segment at `grammar`.
pub(super) fn compress_prose(text: &str, grammar: &Grammar) -> String {
    let (head, tail) = (space_prefix(text), space_suffix(text));
    let end = text.len().saturating_sub(tail);
    let Some(body) = text.get(head..end).filter(|b| !b.is_empty()) else {
        return text.to_owned();
    };
    let out = collapse(&rewrite_words(body, grammar));
    let left = pad(text.get(..head).unwrap_or_default());
    let right = pad(text.get(end..).unwrap_or_default());
    format!("{left}{out}{right}")
}
