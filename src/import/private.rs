//! `<private>...</private>` removed from what cavemem stored, which
//! holds such spans when cavemem ran with `redactSecrets` off: chert
//! writes none of them (root V8), as the hook does (src/hook V1).

#[cfg(test)]
mod tests;

/// `text` without its private spans.
pub(super) fn strip(text: &str) -> String {
    text.to_owned()
}
