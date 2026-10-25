use super::{Piece, pieces};

/// `text` split, each piece as text.
fn split(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    pieces(&chars).map(|(_, of)| of.iter().collect()).collect()
}

fn kinds(text: &str) -> Vec<Piece> {
    let chars: Vec<char> = text.chars().collect();
    pieces(&chars).map(|(piece, _)| piece).collect()
}

#[test]
fn words_take_1_space_or_mark_before_them() {
    assert_eq!(split("fix the (parser"), ["fix", " the", " (", "parser"]);
    assert_eq!(split("a|b"), ["a", "|b"]);
    assert_eq!(split("x\ty"), ["x", "\ty"]);
}

#[test]
fn capitals_split_words_as_o200k_does() {
    assert_eq!(split("camelCase"), ["camel", "Case"]);
    assert_eq!(split("HTTPServer"), ["HTTPServer"]);
    assert_eq!(split("MAX_SIZE"), ["MAX", "_SIZE"]);
    assert_eq!(split("01MH94JKC0"), ["01", "MH", "94", "JKC", "0"]);
}

#[test]
fn contractions_stay_with_their_word() {
    assert_eq!(split("it's"), ["it's"]);
    assert_eq!(split("WE'LL"), ["WE'LL"]);
    assert_eq!(split("they've"), ["they've"]);
    assert_eq!(split("don't"), ["don't"]);
    assert_eq!(split("o'x"), ["o", "'x"]);
}

#[test]
fn digits_go_3_at_a_time_and_take_no_space() {
    assert_eq!(split("1234567"), ["123", "456", "7"]);
    assert_eq!(split("a 1"), ["a", " ", "1"]);
}

#[test]
fn marks_take_1_space_then_line_breaks_and_slashes() {
    assert_eq!(split("a.\n- b"), ["a", ".\n", "-", " b"]);
    assert_eq!(split("x ((y"), ["x", " ((", "y"]);
    assert_eq!(split("x.\n/z"), ["x", ".\n/", "z"]);
}

#[test]
fn spaces_leave_the_last_to_the_next_piece() {
    assert_eq!(split("a   b"), ["a", "  ", " b"]);
    assert_eq!(split("a  "), ["a", "  "]);
    assert_eq!(split("a  \n\n  b"), ["a", "  \n\n", " ", " b"]);
    let found = kinds("a  \n b");
    let want = [Piece::Word, Piece::Break, Piece::Word];
    assert_eq!(found, want);
}

#[test]
fn letters_beyond_ascii_are_words_too() {
    let polish = "za\u{17c}\u{f3}\u{142}\u{107}";
    assert_eq!(
        split(&format!("{polish} {polish}")),
        [polish, &format!(" {polish}")]
    );
    assert_eq!(
        split("\u{dc}n\u{ef}code\u{2192}x"),
        ["\u{dc}n\u{ef}code", "\u{2192}x"]
    );
    assert_eq!(kinds(""), []);
}
