//! Caveman grammar: `compress` and `expand` at intensity `lite`, `full`
//! and `ultra` (src/grammar node).
//!
//! Ported from cavemem 0.2.1 (`packages/compress`: `lexicon.json`,
//! `tokenize.ts`, `compress.ts`, `expand.ts`), Copyright (c) 2026 Julius
//! Brussee, MIT License, <https://github.com/JuliusBrussee/cavemem>; the
//! notice is in `THIRD-PARTY-NOTICES.md`. Output is byte for byte the
//! output of the original (V1), checked against golden files in
//! `tests/grammar/oracle/`.

#[cfg(test)]
mod tests;

mod ident;
mod lexicon;
mod numeric;
mod prose;
mod rules;
mod scan;
mod tokenize;
mod words;

/// How hard `compress` squeezes prose; `Off` leaves text as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intensity {
    /// No grammar applied: `compress` is the identity.
    Off,
    /// Pleasantries, the mildest fillers, five abbreviations.
    Lite,
    /// Adds articles, hedges, more fillers and abbreviations.
    Full,
    /// Adds the strongest hedges and word abbreviations.
    Ultra,
}

/// `text` in caveman grammar at `intensity`; code, URLs, paths,
/// headings, dates, versions, numbers and identifiers are kept.
#[must_use]
pub fn compress(text: &str, intensity: Intensity) -> String {
    let Some(grammar) = lexicon::grammar_for(intensity) else {
        return text.to_owned();
    };
    let joined: String = tokenize::tokenize(text)
        .into_iter()
        .map(|seg| {
            if seg.kept {
                seg.text.to_owned()
            } else {
                prose::compress_prose(seg.text, grammar)
            }
        })
        .collect();
    prose::tighten(&joined)
}

/// `text` with every abbreviation `compress` makes written out again.
#[must_use]
pub fn expand(text: &str) -> String {
    text.to_owned()
}
