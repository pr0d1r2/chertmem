use std::fs;

use super::{PIECES, cutoff, frequent, kept, path, rendered};
use crate::hook::inject::estimate::vocab::blob;
use crate::hook::tests::Test;

/// The drift check (src/hook T14, R8): the committed blob is what o200k
/// and the corpus give; `CHERT_WRITE_TABLE=1` rewrites it first.
#[test]
fn the_committed_vocab_blob_is_generated() -> Test {
    let want = rendered()?;
    if std::env::var_os("CHERT_WRITE_TABLE").is_some() {
        fs::write(path(), &want)?;
    }
    let have = fs::read(path()).unwrap_or_default();
    let stale = "is stale; rewrite it: CHERT_WRITE_TABLE=1 cargo test \
                 --features bpe --lib table::tests";
    assert!(have == want, "{} {stale}", path().display());
    Ok(())
}

/// The proof's premise (R8): every token below the cutoff of its own
/// chars is kept, so a piece finds every token below its cutoff, and
/// every one is found with its dense rank, in o200k's order.
#[test]
fn every_kept_token_is_found_with_its_order() {
    let kept = kept();
    let ranks: Vec<u32> = kept.iter().map(|(_, rank)| *rank).collect();
    assert!(ranks.windows(2).all(|w| w.first() < w.last()));
    for (dense, (token, rank)) in kept.iter().enumerate() {
        assert!(*rank < cutoff(token), "{token:?}");
        let found = blob::rank(token, u32::MAX);
        assert_eq!(found.and_then(|d| usize::try_from(d).ok()), Some(dense));
    }
}

/// Every frequent piece is found with its count.
#[test]
fn every_frequent_piece_is_found_with_its_count() -> Test {
    for (piece, count) in frequent(PIECES)? {
        let found = blob::known(&piece).and_then(|n| u32::try_from(n).ok());
        assert_eq!(found, Some(count), "{piece:?}");
    }
    Ok(())
}
