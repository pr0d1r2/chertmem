use super::{Class, class, count};

#[test]
fn nothing_counts_nothing_and_a_byte_1() {
    assert_eq!(count(b""), 0);
    for byte in 0..=u8::MAX {
        assert_eq!(count(&[byte]), 1, "{byte}");
    }
}

#[test]
fn a_kept_token_counts_1() {
    for token in [&b"hello"[..], b" world", b" the", b"\n\n"] {
        assert_eq!(count(token), 1, "{token:?}");
    }
}

/// o200k merges: 64 `a`s are 8 tokens of 8, so never fewer here.
#[test]
fn a_piece_merges_as_o200k_merges_it() {
    assert!(count(&[b'a'; 64]) >= 8);
}

/// A frequent piece of 2 tokens counts from the pieces table.
#[test]
fn a_frequent_piece_counts_its_tokens() {
    assert_eq!(count(b" chert"), 2);
}

#[test]
fn classes_widen_from_ref_chars_to_anything() {
    assert_eq!(class(b":01MH94JKC0"), Class::Refs);
    assert_eq!(class(b"01I"), Class::Ascii);
    assert_eq!(class(" \u{2192} \u{e9}".as_bytes()), Class::Symbols);
    assert_eq!(class("\u{4e2d}".as_bytes()), Class::Other);
}
