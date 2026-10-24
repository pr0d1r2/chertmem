//! Caveman grammar: `compress` and `expand` at intensity `lite`, `full`
//! and `ultra` (src/grammar node).
//!
//! Ported from cavemem 0.2.1 (`packages/compress`: `lexicon.json`,
//! `tokenize.ts`, `compress.ts`, `expand.ts`), Copyright (c) 2026 Julius
//! Brussee, MIT License, <https://github.com/JuliusBrussee/cavemem>; the
//! notice is in `THIRD-PARTY-NOTICES.md`. Output is byte for byte the
//! output of the original, checked against golden files in
//! `tests/grammar/oracle/`, except three deliberate deviations (V5):
//! blanks before a mark inside code are kept, inline code right after a
//! fence is code, and the blank before a path is kept.

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

use std::borrow::Cow;

use lexicon::Grammar;

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
///
/// cavemem's last pass, blanks before `.,;:!?` dropped, runs over each
/// stretch between fenced code, inline code and paths, never inside
/// code (V5 (a)) nor across into a path that starts with `.` (V5 (c)).
/// Code starts and ends with a backtick or tilde and a path holds no
/// blank, so no other match of the pass crosses their edges: elsewhere
/// the bytes stay the oracle's.
#[must_use]
pub fn compress(text: &str, intensity: Intensity) -> String {
    let Some(grammar) = lexicon::grammar_for(intensity) else {
        return text.to_owned();
    };
    let mut out = String::with_capacity(text.len());
    let mut run = String::new();
    for seg in tokenize::tokenize(text) {
        if seg.code || seg.path {
            out.push_str(&prose::tighten(&std::mem::take(&mut run)));
            out.push_str(seg.text);
        } else {
            run.push_str(&piece(seg, grammar));
        }
    }
    out.push_str(&prose::tighten(&run));
    out
}

/// One segment that is not code: kept as it is, or compressed prose.
fn piece<'a>(seg: tokenize::Segment<'a>, grammar: &Grammar) -> Cow<'a, str> {
    if seg.kept {
        Cow::Borrowed(seg.text)
    } else {
        Cow::Owned(prose::compress_prose(seg.text, grammar))
    }
}

/// `text` with every abbreviation `compress` makes written out again.
#[must_use]
pub fn expand(text: &str) -> String {
    let (keys, targets) = lexicon::expansions();
    let grow = |found: &str, k: usize| {
        let target = targets.get(k).copied().unwrap_or(found);
        words::match_case(found, target)
    };
    tokenize::tokenize(text)
        .into_iter()
        .map(|seg| {
            if seg.kept {
                seg.text.to_owned()
            } else {
                words::replace(seg.text, keys, grow)
            }
        })
        .collect()
}
