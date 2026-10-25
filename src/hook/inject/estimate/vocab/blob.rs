//! `vocab.bin`, read in place (src/hook T14, R8). Big-endian: the
//! dense cutoffs of the ASCII, symbols and other classes (`u32` each),
//! the offset of the pieces table (`u32`), then the vocabulary table and
//! the pieces table. A table: its longest length (`u8`), a directory of (offset,
//! count) `u32` pairs by length, then each length's group: the offset
//! of each block of [`BLOCK`] entries, then the entries, sorted. An
//! entry: a header (the bytes shared with the entry before in its
//! block, above the value bits: a dense rank, or a piece's count), then
//! the bytes not shared. Dense ranks keep o200k's order.

#[cfg(test)]
mod tests;

use std::cmp::Ordering;

use super::{BLOCK, Class, PIECE_BITS, RANK_BITS, class};

/// The generated blob.
static BLOB: &[u8] = include_bytes!("../vocab.bin");

/// Where a table starts, and the bytes and value bits of its headers.
#[derive(Clone, Copy)]
struct Table {
    at: usize,
    head: usize,
    bits: u32,
}

/// The vocabulary table, after the 4 header words.
const VOCAB: Table = Table {
    at: 16,
    head: 3,
    bits: RANK_BITS,
};

/// The dense cutoff of `piece`'s class: none for ref chars, else the
/// header's; 0, merging nothing, should the blob be short.
pub(in crate::hook::inject) fn cutoff(piece: &[u8]) -> u32 {
    let at = match class(piece) {
        Class::Refs => return u32::MAX,
        Class::Ascii => 0,
        Class::Symbols => 4,
        Class::Other => 8,
    };
    read(at, 4).and_then(|n| u32::try_from(n).ok()).unwrap_or(0)
}

/// The dense rank of `token` when kept and below `cut`.
pub(in crate::hook::inject) fn rank(token: &[u8], cut: u32) -> Option<u32> {
    find(VOCAB, token).filter(|&rank| rank < cut)
}

/// The count of `piece` when the pieces table holds it.
pub(in crate::hook::inject) fn known(piece: &[u8]) -> Option<usize> {
    let pieces = Table {
        at: read(12, 4)?,
        head: 2,
        bits: PIECE_BITS,
    };
    find(pieces, piece).and_then(|n| usize::try_from(n).ok())
}

/// The `width` bytes at `at` as a number.
fn read(at: usize, width: usize) -> Option<usize> {
    let bytes = BLOB.get(at..at.checked_add(width)?)?;
    Some(
        bytes
            .iter()
            .fold(0, |n, &b| n.wrapping_shl(8) | usize::from(b)),
    )
}

/// The value `table` holds for `key`.
fn find(table: Table, key: &[u8]) -> Option<u32> {
    let (start, count) = group(table, key.len())?;
    let blocks = count.div_ceil(BLOCK);
    let block = last_at_most(table, start, blocks, key)?;
    let before = block.checked_mul(BLOCK)?;
    let entries = count.saturating_sub(before).min(BLOCK);
    scan(table, offset(start, block)?, entries, key)
}

/// The offset and entry count of `table`'s group of `len` bytes.
fn group(table: Table, len: usize) -> Option<(usize, usize)> {
    let longest = read(table.at, 1)?;
    let entry = len.checked_mul(8)?.checked_add(table.at)?.checked_add(1)?;
    let found = (read(entry, 4)?, read(entry.checked_add(4)?, 4)?);
    (len <= longest).then_some(found)
}

/// The offset of block `block` of the group at `start`.
fn offset(start: usize, block: usize) -> Option<usize> {
    read(block.checked_mul(4)?.checked_add(start)?, 4)
}

/// The last of `blocks` whose first key is at most `key`.
fn last_at_most(
    table: Table,
    start: usize,
    blocks: usize,
    key: &[u8],
) -> Option<usize> {
    let (mut low, mut high) = (0, blocks);
    while low < high {
        let mid = low.midpoint(high);
        let at = offset(start, mid)?.checked_add(table.head)?;
        let first = BLOB.get(at..at.checked_add(key.len())?)?;
        if first <= key {
            low = mid.saturating_add(1);
        } else {
            high = mid;
        }
    }
    low.checked_sub(1)
}

/// Decodes up to `entries` entries from `at` until `key` or past it.
fn scan(
    table: Table,
    mut at: usize,
    entries: usize,
    key: &[u8],
) -> Option<u32> {
    let mut buffer = [0_u8; LONGEST];
    let mask = 1_usize.checked_shl(table.bits)?.saturating_sub(1);
    for _ in 0..entries {
        let head;
        (at, head) = decoded(table, at, key.len(), &mut buffer)?;
        match buffer.get(..key.len())?.cmp(key) {
            Ordering::Equal => return u32::try_from(head & mask).ok(),
            Ordering::Greater => return None,
            Ordering::Less => {}
        }
    }
    None
}

/// The longest kept token or piece, bytes.
const LONGEST: usize = 128;

/// Decodes the entry at `at` of `len` bytes into `buffer`, after the
/// bytes it shares with the one before: where the next starts, and the
/// entry's header.
fn decoded(
    table: Table,
    at: usize,
    len: usize,
    buffer: &mut [u8; LONGEST],
) -> Option<(usize, usize)> {
    let head = read(at, table.head)?;
    let shared = head.checked_shr(table.bits)?;
    let start = at.checked_add(table.head)?;
    let end = start.checked_add(len.checked_sub(shared)?)?;
    let suffix = BLOB.get(start..end)?;
    buffer.get_mut(shared..len)?.copy_from_slice(suffix);
    Some((end, head))
}
