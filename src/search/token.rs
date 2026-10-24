//! The 1 tokenizer every count and score uses: maximal runs of
//! alphanumeric chars, lowercased (src/search R1). Term stats, cache
//! rows and disk rows all go through it, so a row's terms are the same
//! wherever it is read (src/search V1).

#[cfg(test)]
mod tests;

/// Every token of `text`, in order, repeats kept.
pub(super) fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// The distinct tokens of `text`, sorted: a query's terms, or the terms
/// a row counts once each toward document frequency.
pub(super) fn terms(text: &str) -> Vec<String> {
    let mut all = tokens(text);
    all.sort_unstable();
    all.dedup();
    all
}
