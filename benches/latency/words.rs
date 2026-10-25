//! Text for fixtures: words drawn Zipf-like from a coding session's
//! vocabulary, paths shaped like a Rust repository's (benches T2).

#[cfg(test)]
mod tests;

use crate::rng::Rng;

/// The frequent head of the vocabulary, most common first.
const HEAD: [&str; 64] = [
    "the",
    "to",
    "a",
    "test",
    "and",
    "of",
    "in",
    "is",
    "cargo",
    "error",
    "file",
    "build",
    "it",
    "fix",
    "for",
    "run",
    "this",
    "that",
    "fn",
    "with",
    "src",
    "not",
    "add",
    "config",
    "on",
    "search",
    "store",
    "cache",
    "hook",
    "query",
    "path",
    "read",
    "write",
    "check",
    "type",
    "commit",
    "return",
    "value",
    "use",
    "should",
    "now",
    "when",
    "from",
    "partition",
    "session",
    "index",
    "row",
    "spool",
    "clippy",
    "lint",
    "panic",
    "result",
    "option",
    "string",
    "struct",
    "module",
    "trait",
    "async",
    "thread",
    "lock",
    "merge",
    "branch",
    "diff",
    "review",
];

/// Syllables that spell the long tail: rarer words, project names.
const SYLLABLES: [&str; 24] = [
    "ka", "lo", "mi", "ne", "ru", "sa", "to", "vi", "de", "bo", "gu", "ha",
    "ji", "ko", "lu", "ma", "no", "pe", "qi", "re", "si", "tu", "wa", "ze",
];

/// Words in all: the head, then 2- and 3-syllable tail words.
const VOCABULARY: u64 = 4096;

/// The word of `rank` (0 the most common).
pub(crate) fn word(rank: u64) -> String {
    let head = usize::try_from(rank).ok().and_then(|r| HEAD.get(r));
    head.map_or_else(|| tail(rank), |w| (*w).to_owned())
}

/// A tail word spelled from `rank`'s base-24 digits, 2 or 3 of them.
fn tail(rank: u64) -> String {
    // 576 = 24 squared: the 2-syllable words.
    let digits = if rank < 576 { 2 } else { 3 };
    let mut out = String::new();
    let mut rest = rank;
    for _ in 0..digits {
        let i = rest.checked_rem(24).unwrap_or_default();
        let i = usize::try_from(i).unwrap_or_default();
        out.push_str(SYLLABLES.get(i).copied().unwrap_or_default());
        rest = rest.checked_div(24).unwrap_or_default();
    }
    out
}

/// 1 word, Zipf-like.
pub(crate) fn any(rng: &mut Rng) -> String {
    word(rng.skewed(VOCABULARY))
}

/// `n` words joined by spaces.
pub(crate) fn sentence(rng: &mut Rng, n: u64) -> String {
    let words: Vec<String> = (0..n).map(|_| any(rng)).collect();
    words.join(" ")
}

/// A sentence of `min` to `min + spread` words.
pub(crate) fn prose(rng: &mut Rng, min: u64, spread: u64) -> String {
    let n = min.saturating_add(rng.below(spread));
    sentence(rng, n)
}

/// A repository path: a source file most often, else a test or a doc.
pub(crate) fn path(rng: &mut Rng) -> String {
    let (dir, file) = (any(rng), any(rng));
    match rng.below(10) {
        0..=6 => format!("src/{dir}/{file}.rs"),
        7 | 8 => format!("tests/{dir}/{file}.rs"),
        _ => format!("docs/{file}.md"),
    }
}
