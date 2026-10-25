//! The calibration corpus of src/hook T12 (R7): the texts the hook
//! would inject, built from committed files (the grammar corpus, the
//! docs, the README, the hook fixtures) at every grammar intensity, plus
//! the injections of a few seeded stores. Its tests check the default
//! estimate never undercounts o200k on it.

#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{at_start, context, history, past};
use crate::grammar::{Intensity, compress};
use crate::hook::Kind;
use crate::hook::inject::estimate;
use crate::hook::inject::{GIST, HEADER, tokens_at_most};
use crate::hook::tests::{NOW, Res, Scratch};

/// The intensities a summary may be compressed at.
const INTENSITIES: [Intensity; 4] = [
    Intensity::Off,
    Intensity::Lite,
    Intensity::Full,
    Intensity::Ultra,
];

/// Crockford base32, as ULIDs and so refs are written.
const ALPHABET: &[u8] = estimate::ALPHABET.as_bytes();

/// The committed files the corpus is built from, sorted.
fn sources() -> Res<Vec<PathBuf>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut found = vec![root.join("README.md")];
    for (dir, ext) in [
        ("tests/grammar/corpus", "txt"),
        ("docs", "md"),
        ("tests/hook/fixtures/claude-code", "json"),
    ] {
        found.extend(with_ext(&root.join(dir), ext)?);
    }
    found.sort();
    Ok(found)
}

fn with_ext(dir: &Path, ext: &str) -> Res<Vec<PathBuf>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == ext) {
            found.push(path);
        }
    }
    Ok(found)
}

/// `len` base32 chars of `n`, last digit last, as a ULID writes it.
fn base32(mut n: u64, len: usize) -> String {
    let mut chars = vec![b'0'; len];
    for c in chars.iter_mut().rev() {
        let digit = usize::try_from(n.checked_rem(32).unwrap_or(0));
        *c = ALPHABET.get(digit.unwrap_or(0)).copied().unwrap_or(b'0');
        n = n.checked_div(32).unwrap_or(0);
    }
    String::from_utf8_lossy(&chars).into_owned()
}

/// A ref as the store issues them: the first 10 chars of a ULID, its
/// time part, here of the `n`th summary 1 hour apart.
fn reference(n: u64) -> String {
    let ms = NOW.saturating_add(n.saturating_mul(3_600_000));
    format!("chert:{}", base32(ms, 10))
}

/// `text`'s non-blank lines as injected session lines, 3 to a text,
/// each after [`HEADER`], refs counted on from `first`.
fn injected(text: &str, first: u64) -> Vec<String> {
    let gists: Vec<String> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.chars().take(GIST).collect())
        .collect();
    let mut n = first;
    let mut line = |text: String, gist: &String| {
        n = n.saturating_add(1);
        format!("{text}\n- {} {gist}", reference(n))
    };
    let mut out = Vec::new();
    for group in gists.chunks(3) {
        out.push(group.iter().fold(HEADER.to_owned(), &mut line));
    }
    out
}

/// Every source at every intensity, as injected texts.
fn built() -> Res<Vec<String>> {
    let mut out = Vec::new();
    for path in sources()? {
        let text = fs::read_to_string(&path)?;
        for intensity in INTENSITIES {
            let first = u64::try_from(out.len())?.saturating_mul(3);
            out.extend(injected(&compress(&text, intensity), first));
        }
    }
    Ok(out)
}

/// The injections of seeded stores, as the hook prints them.
fn seeded() -> Res<Vec<String>> {
    let long = "Refactored the store and the hook. ".repeat(40);
    let mut out = Vec::new();
    for (n, said) in ["Fixed the parser", long.as_str()].iter().enumerate() {
        for max in [300, 10_000] {
            let mut scratch = Scratch::new(&format!("corpus-{n}-{max}"))?;
            scratch.config.inject.max_tokens = max;
            history(&scratch, 5, said);
            let out_at = at_start(&scratch, "startup");
            out.push(context(out_at, Kind::SessionStart)?);
        }
    }
    out.push(fixtures()?);
    Ok(out)
}

/// The injection after a fixture-like session and 1 saying the README.
fn fixtures() -> Res<String> {
    let mut scratch = Scratch::new("corpus-fixtures")?;
    scratch.config.inject.max_tokens = 10_000;
    past(&scratch, "a", NOW, "chert-fixture");
    let readme = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("README.md");
    let said = fs::read_to_string(readme)?;
    past(&scratch, "b", NOW.saturating_add(10), &said);
    context(at_start(&scratch, "startup"), Kind::SessionStart)
}

/// The whole corpus: built texts, then seeded injections; built once
/// per process, as the seeded stores' dirs are fixed.
pub(super) fn texts() -> Res<&'static [String]> {
    static TEXTS: OnceLock<Result<Vec<String>, String>> = OnceLock::new();
    let built = TEXTS.get_or_init(|| {
        let mut out = built().map_err(|e| e.to_string())?;
        out.extend(seeded().map_err(|e| e.to_string())?);
        Ok(out)
    });
    Ok(built.as_ref().map_err(Clone::clone)?.as_slice())
}

/// Each text's estimate over its exact o200k count, in thousandths, sorted;
/// the text with it.
fn ratios(texts: &[String]) -> Res<Vec<(usize, &String)>> {
    let mut out = Vec::new();
    for text in texts {
        let exact = tokens_at_most(text).max(1);
        let over = estimate::tokens(text).saturating_mul(1_000);
        out.push((over.checked_div(exact).ok_or("no tokens")?, text));
    }
    out.sort();
    Ok(out)
}
/// Prints the totals, the median and worst over-ratio and the worst
/// text (R7: `--nocapture` to read it); the worst, in thousandths.
pub(super) fn report(texts: &[String]) -> Res<usize> {
    let ratios = ratios(texts)?;
    let (worst, text) = ratios.last().ok_or("empty")?;
    let median = ratios.get(ratios.len() / 2).ok_or("empty")?.0;
    let total: usize = texts.iter().map(|t| estimate::tokens(t)).sum();
    let exact: usize = texts.iter().map(|t| tokens_at_most(t)).sum();
    println!("{} texts, {total} counted for {exact}", ratios.len());
    println!("over median {median}, worst {worst}:\n{text}");
    Ok(*worst)
}
/// `count` refs of `len` chars, pseudo-random from `seed` (xorshift).
pub(super) fn random_refs(seed: u64, len: usize, count: usize) -> Vec<String> {
    let mut x = seed;
    let mut next = move || {
        x ^= x.wrapping_shl(13);
        x ^= x.wrapping_shr(7);
        x ^= x.wrapping_shl(17);
        x
    };
    (0..count).map(|_| base32(next(), len)).collect()
}
