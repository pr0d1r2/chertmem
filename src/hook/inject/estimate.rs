//! The default token count of an injected text (src/hook T14, R8):
//! never fewer than o200k's, without loading its table. The text is
//! split as o200k splits it ([`piece`]), each piece counted by o200k's
//! merges over the part of its vocabulary compiled in ([`vocab`]).

#[cfg(test)]
mod tests;

pub(super) mod piece;
pub(super) mod vocab;

/// The tokens `text` counts as: never fewer than o200k's.
pub(crate) fn tokens(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let bytes = text.as_bytes();
    let mut at = 0_usize;
    piece::pieces(&chars).fold(0, |tokens, (_, of)| {
        let start = at;
        at = of.iter().fold(at, |at, c| at.saturating_add(c.len_utf8()));
        let counted = bytes.get(start..at).map_or(usize::MAX, vocab::count);
        tokens.saturating_add(counted)
    })
}
