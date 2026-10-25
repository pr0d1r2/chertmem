use super::tokens;
use crate::hook::inject::HEADER;

#[test]
fn nothing_counts_nothing() {
    assert_eq!(tokens(""), 0);
}

/// o200k's counts (the bpe tests check every corpus text the same way).
#[test]
fn texts_count_as_o200k_counts_them() {
    assert_eq!(tokens("hello world"), 2);
    assert!(tokens(&"a".repeat(64)) >= 8);
    assert_eq!(tokens(HEADER), 12);
    assert_eq!(tokens("- chert:"), 4);
}

#[test]
fn a_ref_counts_by_its_pieces() {
    // `-`, ` chert`, `:`, `01`, `MH`, `94`, `JKC`, `0`, ` fix`.
    let counted = tokens("- chert:01MH94JKC0 fix");
    assert!(counted >= 9, "{counted}");
    assert_eq!(tokens("- chert:01MH94JKC0 fix"), counted);
}

#[test]
fn every_piece_counts_1_token_at_least() {
    for (text, pieces) in [("a b c", 3), (".,;:!", 1), ("x  y   z", 5)] {
        assert!(tokens(text) >= pieces, "{text}");
    }
}

#[test]
fn a_text_not_ascii_counts_1_token_at_least_per_piece() {
    assert!(tokens(" \u{e9}") >= 1);
    assert!(tokens("\u{1f600}\u{1f600}") >= 1);
}
