use super::{cutoff, known, rank};

/// Ref chars merge with every kept rank, wider classes below lower
/// cutoffs, each past the bytes.
#[test]
fn cutoffs_fall_as_classes_widen() {
    let refs = cutoff(b":A1");
    let ascii = cutoff(b"a");
    let symbols = cutoff("\u{e9}".as_bytes());
    let other = cutoff("\u{4e2d}".as_bytes());
    assert_eq!(refs, u32::MAX);
    assert!(ascii > symbols && symbols > other, "{ascii} {symbols}");
    assert!(other > 255, "{other}");
}

/// Bytes are o200k's first ranks, kept in its order.
#[test]
fn every_byte_is_kept_in_order() {
    let ranks: Vec<Option<u32>> =
        (0..=u8::MAX).map(|b| rank(&[b], u32::MAX)).collect();
    assert!(ranks.iter().all(Option::is_some));
    let mut sorted = ranks.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 256);
}

/// A rank at or past the cutoff is not found.
#[test]
fn a_rank_past_the_cutoff_is_not_found() {
    let hello = rank(b"hello", u32::MAX);
    assert!(hello.is_some());
    assert_eq!(hello.and_then(|r| rank(b"hello", r)), None);
}

#[test]
fn what_is_not_kept_is_not_found() {
    assert_eq!(rank(b"zqzqzqzq", u32::MAX), None);
    assert_eq!(rank(&[b'x'; 300], u32::MAX), None);
    assert_eq!(known(b"hello"), None);
    assert_eq!(known(b" chert"), Some(2));
}
