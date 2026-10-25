//! The default token count of an injected text (src/hook T12, R7): o200k
//! never undercounted on the corpus, without its table. RED stub: still
//! bytes / 4, rounded up.

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(super) mod piece;

#[cfg(test)]
use super::HEADER;

/// The o200k tokens of [`HEADER`] (asserted by the bpe tests).
#[cfg(test)]
pub(super) const HEADER_TOKENS: usize = HEADER.len().div_ceil(4);

/// What starts a session line with a ref.
#[cfg(test)]
pub(super) const LINE: &str = "- chert:";

/// The o200k tokens of [`LINE`] at a line start (bpe tests).
#[cfg(test)]
pub(super) const LINE_TOKENS: usize = LINE.len().div_ceil(4);

/// Crockford base32, as refs are written.
#[cfg(all(test, feature = "bpe"))]
pub(super) const ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// The tokens `text` counts as: never fewer than o200k's on the corpus.
pub(crate) fn tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

/// The most o200k tokens `- chert:<reference>` takes.
#[cfg(test)]
pub(super) fn reference_tokens(reference: &str) -> usize {
    LINE_TOKENS.saturating_add(tokens(reference))
}
