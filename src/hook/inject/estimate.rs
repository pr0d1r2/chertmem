//! The default token count of an injected text (src/hook T12, R7): o200k
//! never undercounted on the corpus, without its table. [`HEADER`] and
//! each `- chert:<ref>` count by their fixed shape; the rest (gists,
//! line breaks) is split as o200k splits it, each piece weighed by its
//! class and size in sixteenths of a token, the sum rounded up.

#[cfg(test)]
mod tests;

pub(super) mod piece;

use piece::Piece;

use super::HEADER;

/// The o200k tokens of [`HEADER`] (asserted by the bpe tests).
pub(super) const HEADER_TOKENS: usize = 12;

/// What starts a session line with a ref.
pub(super) const LINE: &str = "- chert:";

/// The o200k tokens of [`LINE`] at a line start (bpe tests).
pub(super) const LINE_TOKENS: usize = 4;

/// Ref lengths a line may carry (src/store: 8 chars up to a ULID).
const REF: std::ops::RangeInclusive<usize> = 8..=26;

/// Crockford base32, as refs are written.
pub(super) const ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// The fixed point of the weights: sixteenths of a token.
const SCALE: usize = 16;

/// A word piece, by its lead (a space then a small letter, a space then
/// a capital, anything else) and its letters (1, 2-3, 4-5, 6-8, 9-12,
/// 13+). Fitted on the corpus (R7), every piece 1 token at least, a
/// longer word never less.
const WORDS: [[usize; 6]; 3] = [
    [16, 18, 18, 25, 25, 25],
    [16, 16, 16, 16, 37, 37],
    [23, 26, 26, 26, 26, 26],
];

/// A marks piece, by its lead (none, a space) and its marks (1, 2,
/// 3-4, 5+), fitted as [`WORDS`].
const MARKS: [[usize; 4]; 2] = [[20, 20, 20, 20], [16, 16, 22, 22]];

/// A digits, line-break or spaces piece.
const PIECE: usize = 16;

/// Each non-ASCII byte, on top of its piece.
const WIDE: usize = 8;

/// The tokens `text` counts as: never fewer than o200k's on the corpus.
pub(crate) fn tokens(text: &str) -> usize {
    let (mut fixed, mut weight) = (0_usize, 0_usize);
    for (n, line) in text.split_inclusive('\n').enumerate() {
        let (tokens, rest) = shape(line, n == 0);
        fixed = fixed.saturating_add(tokens);
        let chars: Vec<char> = rest.chars().collect();
        for (piece, of) in piece::pieces(&chars) {
            weight = weight.saturating_add(weigh(piece, of));
        }
    }
    fixed.saturating_add(weight.div_ceil(SCALE))
}

/// The tokens of the fixed shape leading `line`, and the rest of it:
/// [`HEADER`] as the 1st line, or [`LINE`] and a ref.
fn shape(line: &str, first: bool) -> (usize, &str) {
    let header = line.strip_prefix(HEADER);
    if let Some(rest) = header.filter(|r| first && matches!(*r, "" | "\n")) {
        return (HEADER_TOKENS, rest);
    }
    line.strip_prefix(LINE)
        .and_then(reference)
        .unwrap_or((0, line))
}

/// The tokens of [`LINE`] and the ref leading `after`, and the rest,
/// when a ref of [`REF`] chars ends at a space or the line end.
fn reference(after: &str) -> Option<(usize, &str)> {
    let len = after.find(|c| !ALPHABET.contains(c)).unwrap_or(after.len());
    let (reference, rest) = after.split_at_checked(len)?;
    let ends = matches!(rest.chars().next(), None | Some(' ' | '\n'));
    (ends && REF.contains(&len)).then(|| (reference_tokens(reference), rest))
}

/// The most o200k tokens `- chert:<reference>` takes: [`LINE`], then 1
/// per run of 1 to 3 digits and 1 per capital, the most a run of
/// capitals takes (bpe tests).
pub(super) fn reference_tokens(reference: &str) -> usize {
    let chars: Vec<char> = reference.chars().collect();
    piece::pieces(&chars).fold(LINE_TOKENS, |tokens, (piece, of)| {
        let most = if piece == Piece::Number { 1 } else { of.len() };
        tokens.saturating_add(most)
    })
}

/// The sixteenths of a token `chars`, 1 piece, counts as.
fn weigh(piece: Piece, chars: &[char]) -> usize {
    let wide: usize = chars
        .iter()
        .filter(|c| !c.is_ascii())
        .map(|c| c.len_utf8())
        .sum();
    let base = match piece {
        Piece::Word => word(chars),
        Piece::Marks => marks(chars),
        Piece::Number | Piece::Break | Piece::Spaces => PIECE,
    };
    base.saturating_add(WIDE.saturating_mul(wide))
}

/// Letters a word piece's size grows past: 1, 2-3, 4-5, 6-8, 9-12, 13+.
const WORD_SIZES: [usize; 5] = [1, 3, 5, 8, 12];

/// Marks a marks piece's size grows past: 1, 2, 3-4, 5+.
const MARKS_SIZES: [usize; 3] = [1, 2, 4];

fn word(chars: &[char]) -> usize {
    let letters = chars.iter().filter(|c| c.is_alphabetic()).count();
    let mut alphabetic = chars.iter().filter(|c| c.is_alphabetic());
    let capital = alphabetic.next().is_some_and(|c| c.is_uppercase());
    let lead = match chars.first() {
        Some(' ') => usize::from(capital),
        _ => 2,
    };
    let row = WORDS.get(lead);
    let weight = row.and_then(|row| row.get(size(letters, &WORD_SIZES)));
    weight.copied().unwrap_or(usize::MAX)
}

fn marks(chars: &[char]) -> usize {
    let lead = usize::from(chars.first() == Some(&' '));
    let body = chars.get(lead..).unwrap_or_default();
    let marks = body.iter().take_while(|&&c| piece::mark(c)).count();
    let row = MARKS.get(lead);
    let weight = row.and_then(|row| row.get(size(marks, &MARKS_SIZES)));
    weight.copied().unwrap_or(usize::MAX)
}

/// The size class of `n`: how many of `sizes` it is past.
fn size(n: usize, sizes: &[usize]) -> usize {
    sizes.iter().filter(|&&past| n > past).count()
}
