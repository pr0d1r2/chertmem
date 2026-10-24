//! Byte and char probes with JavaScript regex meaning: `\b`, `\w`, `\d`
//! are ASCII (no `u` flag), `\s` is the ECMAScript white space set, and
//! `^`/`$` under `m` know four line terminators.

#[cfg(test)]
mod tests;

/// The byte at `i`, if any.
pub(super) fn at(b: &[u8], i: usize) -> Option<u8> {
    b.get(i).copied()
}

/// `\w`: an ASCII letter, digit or `_`.
pub(super) fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// Whether the byte at `i` is a `\w`.
pub(super) fn word_at(b: &[u8], i: usize) -> bool {
    at(b, i).is_some_and(is_word)
}

/// `\b` at `i`: a `\w` on exactly one side.
pub(super) fn boundary(b: &[u8], i: usize) -> bool {
    let before = i.checked_sub(1).is_some_and(|j| word_at(b, j));
    before != word_at(b, i)
}

/// The end of the run of bytes from `i` that satisfy `keep`.
pub(super) fn run(b: &[u8], i: usize, keep: impl Fn(u8) -> bool) -> usize {
    let len = b
        .get(i..)
        .map_or(0, |t| t.iter().take_while(|&&c| keep(c)).count());
    i.saturating_add(len)
}

/// The index after byte `c` at `i`, if `c` is there.
pub(super) fn eat(b: &[u8], i: usize, c: u8) -> Option<usize> {
    (at(b, i) == Some(c)).then(|| i.saturating_add(1))
}

/// The index after exactly `n` ASCII digits from `i`, if they are there.
pub(super) fn digits(b: &[u8], i: usize, n: usize) -> Option<usize> {
    let end = i.checked_add(n)?;
    (run(b, i, |c| c.is_ascii_digit()) >= end).then_some(end)
}

/// The index after one or more ASCII digits from `i`.
pub(super) fn some_digits(b: &[u8], i: usize) -> Option<usize> {
    let end = run(b, i, |c| c.is_ascii_digit());
    (end > i).then_some(end)
}

/// The char starting at byte `i` of `s`, if `i` starts one.
pub(super) fn char_at(s: &str, i: usize) -> Option<char> {
    s.get(i..)?.chars().next()
}

/// `\s` in ECMAScript: white space and line terminators.
pub(super) fn js_space(c: char) -> bool {
    (c.is_whitespace() && c != '\u{85}') || c == '\u{feff}'
}

/// A line terminator for `^` and `$` under the `m` flag.
pub(super) fn line_end(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// `^` under `m` at byte `i`: the start, or just after a line terminator.
pub(super) fn line_start(s: &str, i: usize) -> bool {
    let before = s.get(..i).and_then(|t| t.chars().next_back());
    i == 0 || before.is_some_and(line_end)
}

/// The length in bytes of the run of `\s` at the start of `s`.
pub(super) fn space_prefix(s: &str) -> usize {
    s.len().saturating_sub(s.trim_start_matches(js_space).len())
}

/// The length in bytes of the run of `\s` at the end of `s`.
pub(super) fn space_suffix(s: &str) -> usize {
    s.len().saturating_sub(s.trim_end_matches(js_space).len())
}
