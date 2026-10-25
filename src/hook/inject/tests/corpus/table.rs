//! The generator of `estimate/vocab.bin` (src/hook T14, R8): the part
//! of o200k's vocabulary the default count merges with, read from
//! tiktoken-rs, and the most frequent pieces of the memory-shaped
//! corpus it does not count alone, front-coded into 1 binary blob. Its
//! test is the drift check; `CHERT_WRITE_TABLE=1` rewrites the blob.

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::{SNAPSHOT, built_from, sources};
use crate::hook::inject::estimate::piece;
use crate::hook::inject::estimate::vocab::{
    BLOCK, Class, PIECE_BITS, RANK_BITS, class,
};
use crate::hook::inject::tokens_at_most;
use crate::hook::tests::Res;

/// The rank below which every ASCII token is kept (R8: the smallest
/// keeping the held-out sources within 1.03, of 98k, 110k, 130k, 140k,
/// 150k, 155k, 160k and all).
pub(super) const ASCII: u32 = 160_000;

/// The rank below which every token of ASCII, Latin-1 and symbol bytes
/// is kept (R8: covers the spec symbols of the held-out sources).
pub(super) const SYMBOLS: u32 = 45_000;

/// The rank below which every other token is kept.
pub(super) const OTHER: u32 = 4_000;

/// Past every rank `o200k_base` holds, its special tokens included.
const RANKS: u32 = 201_000;

/// Pieces in the pieces table at most.
pub(super) const PIECES: usize = 1_024;

/// The longest piece the pieces table holds.
const PIECE_LEN: usize = 64;

/// A table's rows: a byte string and its value, grouped by length, each
/// group sorted.
pub(super) type Groups = Vec<Vec<(Vec<u8>, u32)>>;

/// The committed blob's path.
pub(super) fn path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/hook/inject/estimate/vocab.bin")
}

/// The rank below which `token` is kept, by its class.
pub(super) fn cutoff(token: &[u8]) -> u32 {
    match class(token) {
        Class::Refs => u32::MAX,
        Class::Ascii => ASCII,
        Class::Symbols => SYMBOLS,
        Class::Other => OTHER,
    }
}

/// Every kept token and its o200k rank, by o200k rank.
pub(super) fn kept() -> Vec<(Vec<u8>, u32)> {
    let bpe = tiktoken_rs::o200k_base_singleton();
    let special: HashSet<&str> = bpe.special_tokens();
    let mut out = Vec::new();
    for rank in 0..RANKS {
        let Ok(token) = bpe.decode_bytes(&[rank]) else {
            continue;
        };
        let named =
            std::str::from_utf8(&token).is_ok_and(|t| special.contains(t));
        if !named && rank < cutoff(&token) {
            out.push((token, rank));
        }
    }
    out
}

/// The blob: 3 dense cutoffs, the pieces table's offset, then the
/// vocabulary (dense ranks) and the pieces (counts), each a directory
/// by length and front-coded groups.
pub(super) fn rendered() -> Res<Vec<u8>> {
    let kept = kept();
    let mut out = Vec::new();
    for below in [ASCII, SYMBOLS, OTHER] {
        let dense = kept.iter().filter(|(_, r)| *r < below).count();
        out.extend(u32::try_from(dense)?.to_be_bytes());
    }
    out.extend([0; 4]);
    let ranked = kept.into_iter().enumerate();
    let vocab = ranked.map(|(n, (token, _))| Ok((token, u32::try_from(n)?)));
    let vocab = grouped(vocab.collect::<Res<Vec<_>>>()?);
    table(&vocab, RANK_BITS, &mut out)?;
    mark(&mut out, 12)?;
    table(&grouped(frequent(PIECES)?), PIECE_BITS, &mut out)?;
    Ok(out)
}

/// Writes the length of `out` big-endian at `at`: where what comes
/// next starts.
fn mark(out: &mut [u8], at: usize) -> Res<()> {
    let end = u32::try_from(out.len())?;
    put(out, at, end)
}

/// Writes `n` big-endian at `at`.
fn put(out: &mut [u8], at: usize, n: u32) -> Res<()> {
    let slot = out
        .get_mut(at..at.saturating_add(4))
        .ok_or("past the end")?;
    slot.copy_from_slice(&n.to_be_bytes());
    Ok(())
}

fn grouped(rows: Vec<(Vec<u8>, u32)>) -> Groups {
    let mut groups: Groups = Vec::new();
    for (key, value) in rows {
        let len = key.len();
        if groups.len() <= len {
            groups.resize_with(len.saturating_add(1), Vec::new);
        }
        if let Some(group) = groups.get_mut(len) {
            group.push((key, value));
        }
    }
    for group in &mut groups {
        group.sort();
    }
    groups
}

/// A table at the end of `out`: its longest length, a directory entry
/// (offset, count) per length, then each group.
fn table(groups: &Groups, bits: u32, out: &mut Vec<u8>) -> Res<()> {
    out.push(u8::try_from(groups.len().saturating_sub(1))?);
    let directory = out.len();
    out.resize(directory.saturating_add(groups.len().saturating_mul(8)), 0);
    for (len, group) in groups.iter().enumerate() {
        let at = directory.saturating_add(len.saturating_mul(8));
        mark(out, at)?;
        put(out, at.saturating_add(4), u32::try_from(group.len())?)?;
        coded(group, bits, out)?;
    }
    Ok(())
}

/// 1 group: the offset of each block of [`BLOCK`] entries, then the
/// entries, each a header (the bytes shared with the entry before in
/// its block, above `bits` bits of value) and the bytes not shared.
fn coded(group: &[(Vec<u8>, u32)], bits: u32, out: &mut Vec<u8>) -> Res<()> {
    let index = out.len();
    let blocks = group.len().div_ceil(BLOCK);
    out.resize(index.saturating_add(blocks.saturating_mul(4)), 0);
    for (block, rows) in group.chunks(BLOCK).enumerate() {
        let at = index.saturating_add(block.saturating_mul(4));
        mark(out, at)?;
        let mut before: &[u8] = &[];
        for (key, value) in rows {
            entry((key, before), *value, bits, out)?;
            before = key;
        }
    }
    Ok(())
}

/// 1 entry, `key` after `before`: its header in the table's width (the
/// shared length above `bits` bits of `value`), then the unshared bytes.
fn entry(
    (key, before): (&[u8], &[u8]),
    value: u32,
    bits: u32,
    out: &mut Vec<u8>,
) -> Res<()> {
    let shared = key.iter().zip(before).take_while(|(a, b)| a == b).count();
    let head = u32::try_from(shared)?.checked_shl(bits).ok_or("shared")?;
    let width = if bits == RANK_BITS { 3 } else { 2 };
    let be = (head | value).to_be_bytes();
    out.extend(be.get(be.len().saturating_sub(width)..).unwrap_or_default());
    out.extend(key.get(shared..).unwrap_or_default());
    Ok(())
}

/// The `n` most frequent pieces of the memory-shaped corpus, the texts
/// of its committed test inputs (the grammar corpus and the hook
/// fixtures, at every intensity), that o200k splits into 2 to 255
/// tokens, ties by bytes, with their counts: counted without merging,
/// and exactly where the vocabulary alone would count more.
pub(super) fn frequent(n: usize) -> Res<Vec<(Vec<u8>, u32)>> {
    let mut out = Vec::new();
    for piece in ranked()? {
        let count = u32::try_from(tokens_at_most(&piece))?;
        let fits = piece.len() <= PIECE_LEN && (2..=255).contains(&count);
        if fits && out.len() < n {
            out.push((piece.into_bytes(), count));
        }
    }
    Ok(out)
}

/// Every piece of the memory-shaped corpus, the most frequent first.
fn ranked() -> Res<Vec<String>> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    for text in built_from(&inputs()?)? {
        let chars: Vec<char> = text.chars().collect();
        for (_, of) in piece::pieces(&chars) {
            let times = seen.entry(of.iter().collect()).or_default();
            *times = times.saturating_add(1);
        }
    }
    let mut ranked: Vec<(usize, String)> =
        seen.into_iter().map(|(piece, n)| (n, piece)).collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    Ok(ranked.into_iter().map(|(_, piece)| piece).collect())
}

/// The grammar corpus and the hook fixtures: test inputs, edited
/// rarely; the docs snapshot stays out, read only to measure.
fn inputs() -> Res<Vec<PathBuf>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = root.join(SNAPSHOT);
    let all = sources()?.into_iter();
    Ok(all.filter(|path| !path.starts_with(&docs)).collect())
}
