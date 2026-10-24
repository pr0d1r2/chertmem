//! `<private>...</private>` removed before anything is compressed or
//! written (src/hook V1, root V8).
//!
//! Tags match in any ASCII case; a span ends at the first close tag; an
//! open tag never closed drops everything after it, so a cut-off payload
//! leaks nothing.

#[cfg(test)]
mod tests;

const OPEN: &[u8] = b"<private>";
const CLOSE: &[u8] = b"</private>";

/// `text` without its private spans.
pub(super) fn strip(text: &str) -> String {
    // Spans start and end on ASCII bytes, so what is left stays UTF-8.
    String::from_utf8_lossy(&strip_bytes(text.as_bytes())).into_owned()
}

/// `raw` without its private spans, whatever its encoding.
fn strip_bytes(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(at) = find(rest, OPEN) {
        out.extend_from_slice(rest.get(..at).unwrap_or_default());
        rest = after_close(rest.get(at..).unwrap_or_default());
    }
    out.extend_from_slice(rest);
    out
}

/// Where `needle` first starts in `hay`, in any ASCII case.
fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len())
        .position(|w| w.eq_ignore_ascii_case(needle))
}

/// What follows the first close tag in `span`; nothing when unclosed.
fn after_close(span: &[u8]) -> &[u8] {
    let end = find(span, CLOSE).map(|at| at.saturating_add(CLOSE.len()));
    end.and_then(|end| span.get(end..)).unwrap_or_default()
}
