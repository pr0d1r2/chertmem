//! `<private>...</private>` removed from what cavemem stored, which
//! holds such spans when cavemem ran with `redactSecrets` off: chert
//! writes none of them (root V8), with the hook's rules (src/hook V1):
//! tags in any ASCII case, a span ends at the first close tag, an open
//! tag never closed drops everything after it.

#[cfg(test)]
mod tests;

const OPEN: &[u8] = b"<private>";
const CLOSE: &[u8] = b"</private>";

/// `text` without its private spans.
pub(super) fn strip(text: &str) -> String {
    let mut out = Vec::with_capacity(text.len());
    let mut rest = text.as_bytes();
    while let Some(at) = find(rest, OPEN) {
        out.extend_from_slice(rest.get(..at).unwrap_or_default());
        rest = after_close(rest.get(at..).unwrap_or_default());
    }
    out.extend_from_slice(rest);
    // Spans start and end on ASCII bytes, so what is left stays UTF-8.
    String::from_utf8_lossy(&out).into_owned()
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
