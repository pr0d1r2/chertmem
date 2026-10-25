use super::piece::pieces;
use super::{HEADER, HEADER_TOKENS, LINE_TOKENS, reference_tokens, tokens};

#[test]
fn nothing_counts_nothing() {
    assert_eq!(tokens(""), 0);
}

#[test]
fn the_header_counts_its_constant_as_the_1st_line_only() {
    assert_eq!(tokens(HEADER), HEADER_TOKENS);
    let broken = HEADER_TOKENS.saturating_add(1);
    assert_eq!(tokens(&format!("{HEADER}\n")), broken);
    assert_ne!(tokens(&format!("{HEADER}x")), HEADER_TOKENS);
    let twice = tokens(&format!("{HEADER}\n{HEADER}"));
    assert_ne!(twice, broken.saturating_add(HEADER_TOKENS));
}

#[test]
fn a_ref_counts_its_line_start_digit_runs_and_capitals() {
    // `01`, `MH`, `94`, `JKC`, `0`: 3 digit runs, 5 capitals.
    assert_eq!(reference_tokens("01MH94JKC0"), LINE_TOKENS + 8);
    assert_eq!(reference_tokens("01234567"), LINE_TOKENS + 3);
    assert_eq!(tokens("- chert:01MH94JKC0"), LINE_TOKENS + 8);
}

#[test]
fn a_gist_after_a_ref_counts_by_its_pieces() {
    // ` fix`: a space, 3 small letters, 18 sixteenths of a token.
    assert_eq!(tokens("- chert:01MH94JKC0 fix"), LINE_TOKENS + 10);
}

#[test]
fn what_is_no_ref_counts_by_its_pieces() {
    // `-` 20, ` chert` 18, `:` 20, `01` 16, `MH` 26, `94` 16, `J` 23,
    // ` fix` 18: 157 sixteenths, 10 tokens.
    assert_eq!(tokens("- chert:01MH94J fix"), 10);
    // As a ref, 7; as pieces, `-` 20, ` chert` 18, `:` 20, 3 digit runs
    // of 16 and `x` 23: 129 sixteenths, 9 tokens.
    assert_eq!(tokens("- chert:01234567"), LINE_TOKENS + 3);
    assert_eq!(tokens("- chert:01234567x"), 9);
    assert_eq!(tokens("x- chert:01234567"), 9);
}

#[test]
fn every_piece_counts_1_token_at_least() {
    for text in ["a b c", "1 2 3 4", ".,;:!", "a\n\nb\n", "x  y   z"] {
        let chars: Vec<char> = text.chars().collect();
        assert!(tokens(text) >= pieces(&chars).count(), "{text}");
    }
}

#[test]
fn a_longer_piece_never_counts_less() {
    for (lead, letter) in [(" ", "a"), (" A", "b"), ("(", "c"), (" ", ".")] {
        let mut last = 0;
        for len in 0..20 {
            let text = format!("{lead}{}", letter.repeat(len));
            assert!(tokens(&text) >= last, "{text}");
            last = tokens(&text);
        }
    }
}

#[test]
fn each_non_ascii_byte_adds_half_a_token() {
    // ` e` with an acute accent: 16 for the word, 2 bytes of 8.
    assert_eq!(tokens(" \u{e9}"), 2);
    assert_eq!(tokens(" e"), 1);
}
