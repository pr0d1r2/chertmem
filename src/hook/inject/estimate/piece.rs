//! o200k's split of a text into pieces before its BPE, which never
//! merges across 2 pieces: its regex, alternative by alternative
//! (tiktoken `o200k_base`), with letters as Rust's alphabetic chars,
//! capitals as its uppercase ones, the rest of them small.

#[cfg(test)]
mod tests;

/// What a piece is, by the alternative that matched it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::hook::inject) enum Piece {
    /// Letters, maybe after 1 space or mark, maybe with `'s` and kin.
    Word,
    /// 1 to 3 digits.
    Number,
    /// Marks, maybe after 1 space, then any line breaks or slashes.
    Marks,
    /// Spaces up to and including line breaks.
    Break,
    /// Spaces before a word, the end or nothing else.
    Spaces,
}

/// The pieces of `chars`, in order, each with its chars.
pub(in crate::hook::inject) fn pieces(
    mut rest: &[char],
) -> impl Iterator<Item = (Piece, &[char])> {
    std::iter::from_fn(move || {
        let (piece, len) = next(rest)?;
        let (now, after) = rest.split_at_checked(len).unwrap_or((rest, &[]));
        rest = after;
        Some((piece, now))
    })
}

/// The first piece of `rest` and its length in chars, 1 at least;
/// `None` when `rest` is empty.
fn next(rest: &[char]) -> Option<(Piece, usize)> {
    rest.first()?;
    let found = word(rest)
        .map(|n| (Piece::Word, n))
        .or_else(|| number(rest).map(|n| (Piece::Number, n)))
        .or_else(|| marks(rest).map(|n| (Piece::Marks, n)))
        .or_else(|| breaks(rest).map(|n| (Piece::Break, n)));
    Some(found.unwrap_or_else(|| (Piece::Spaces, spaces(rest))))
}

fn letter(c: char) -> bool {
    c.is_alphabetic()
}

fn number_char(c: char) -> bool {
    c.is_numeric()
}

fn newline(c: char) -> bool {
    matches!(c, '\r' | '\n')
}

/// Neither space, letter nor digit.
pub(super) fn mark(c: char) -> bool {
    !c.is_whitespace() && !letter(c) && !number_char(c)
}

/// Capitals then small letters, or capitals alone, after at most 1
/// char that is no letter, digit or line break.
fn word(rest: &[char]) -> Option<usize> {
    let lead = rest
        .first()
        .is_some_and(|&c| !letter(c) && !number_char(c) && !newline(c));
    let body = rest.get(usize::from(lead)..)?;
    let caps = run(body, |c| letter(c) && c.is_uppercase());
    let small = run(body.get(caps..)?, |c| letter(c) && !c.is_uppercase());
    let len = caps.checked_add(small).filter(|&n| n > 0)?;
    let len = len.checked_add(usize::from(lead))?;
    len.checked_add(contraction(rest.get(len..)?))
}

/// `'s`, `'t`, `'re`, `'ve`, `'m`, `'ll` or `'d` in any case: its length.
fn contraction(rest: &[char]) -> usize {
    let lower: String =
        rest.iter().take(3).flat_map(|c| c.to_lowercase()).collect();
    let two = ["'s", "'t", "'m", "'d"]
        .iter()
        .any(|s| lower.starts_with(s));
    let three = ["'re", "'ve", "'ll"].iter().any(|s| lower.starts_with(s));
    if three {
        3
    } else {
        usize::from(two).saturating_mul(2)
    }
}

fn number(rest: &[char]) -> Option<usize> {
    Some(run(rest, number_char).min(3)).filter(|&n| n > 0)
}

/// Marks after at most 1 space, then line breaks and slashes.
fn marks(rest: &[char]) -> Option<usize> {
    let lead = usize::from(rest.first() == Some(&' '));
    let body = rest.get(lead..)?;
    let marks = Some(run(body, mark)).filter(|&n| n > 0)?;
    let tail = run(body.get(marks..)?, |c| newline(c) || c == '/');
    lead.checked_add(marks)?.checked_add(tail)
}

/// Spaces through the last line break among them.
fn breaks(rest: &[char]) -> Option<usize> {
    let spaces = rest.get(..run(rest, char::is_whitespace))?;
    spaces.iter().rposition(|&c| newline(c))?.checked_add(1)
}

/// Spaces without a line break: all of them at the end, else all but
/// the last, which leads the next piece; 1 at least.
fn spaces(rest: &[char]) -> usize {
    let len = run(rest, char::is_whitespace);
    if len == rest.len() {
        len
    } else {
        len.saturating_sub(1).max(1)
    }
}

/// The leading chars of `rest` that are `is`.
fn run(rest: &[char], is: impl Fn(char) -> bool) -> usize {
    rest.iter().take_while(|&&c| is(c)).count()
}
