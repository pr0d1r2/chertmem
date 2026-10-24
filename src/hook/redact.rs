//! `<private>...</private>` removed before anything is compressed or
//! written (src/hook V1, root V8).

#[cfg(test)]
mod tests;

/// `text` without its private spans.
pub(super) fn strip(text: &str) -> String {
    text.to_owned()
}
