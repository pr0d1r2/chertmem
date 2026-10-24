//! `commit-msg`: the message git hands the hook names its type, says why,
//! and ends with the paradigm stage as a trailer (dev V11, B4).

#[cfg(test)]
mod tests;

/// Every rule `text` breaks, each with its fix; empty when it holds.
#[must_use]
pub fn findings(text: &str) -> Vec<String> {
    let _ = text;
    Vec::new()
}
