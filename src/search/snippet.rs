//! A hit's snippet: [`WIDTH`] chars of the body, starting a little
//! before the first token that matches a query term.

#[cfg(test)]
mod tests;

/// Chars a snippet holds at most.
pub(super) const WIDTH: usize = 120;
/// Chars kept before the first match.
const LEAD: usize = 30;

/// The snippet of `body` for the distinct, lowercased `terms`.
pub(super) fn around(body: &str, terms: &[String]) -> String {
    let start = first_match(body, terms).saturating_sub(LEAD);
    body.chars().skip(start).take(WIDTH).collect()
}

/// The char offset of the first token in `terms`; 0 when none is.
fn first_match(body: &str, terms: &[String]) -> usize {
    let mut at = 0_usize;
    let cut = |c: char| !c.is_alphanumeric();
    for piece in body.split_inclusive(cut) {
        let word = piece.trim_end_matches(cut).to_lowercase();
        if !word.is_empty() && terms.contains(&word) {
            return at;
        }
        at = at.saturating_add(piece.chars().count());
    }
    0
}
