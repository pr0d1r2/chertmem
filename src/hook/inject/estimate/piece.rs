//! o200k's split of a text into pieces before its BPE. RED stub: the
//! whole text is 1 word.

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
    chars: &[char],
) -> impl Iterator<Item = (Piece, &[char])> {
    let kinds = [
        Piece::Word,
        Piece::Number,
        Piece::Marks,
        Piece::Break,
        Piece::Spaces,
    ];
    let kind = kinds.into_iter().find(|_| !chars.is_empty());
    kind.map(|piece| (piece, chars)).into_iter()
}
