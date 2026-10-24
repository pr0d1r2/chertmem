use super::{
    boundary, char_at, digits, js_space, line_end, line_start, some_digits,
    space_prefix, space_suffix,
};

#[test]
fn boundary_is_ascii_word_on_one_side() {
    let b = "a b_\u{e9}".as_bytes();
    assert!(boundary(b, 0));
    assert!(boundary(b, 1));
    assert!(!boundary(b, 3));
    assert!(boundary(b, 4));
    assert!(!boundary(b, 5));
    assert!(!boundary(b"", 0));
}

#[test]
fn digits_are_ascii_and_counted() {
    assert_eq!(digits(b"2024x", 0, 4), Some(4));
    assert_eq!(digits(b"202x", 0, 4), None);
    assert_eq!(some_digits(b"x12y", 1), Some(3));
    assert_eq!(some_digits(b"x12y", 0), None);
}

#[test]
fn js_white_space_differs_from_rust_at_two_chars() {
    assert!(js_space('\u{feff}'));
    assert!(!js_space('\u{85}'));
    assert!(js_space('\u{3000}') && js_space('\u{b}') && js_space('\u{a0}'));
    assert!(!js_space('x'));
}

#[test]
fn line_terminators_are_four() {
    let ends = ['\n', '\r', '\u{2028}', '\u{2029}'];
    assert!(ends.into_iter().all(line_end));
    assert!(!line_end('\u{85}') && !line_end(' '));
}

#[test]
fn line_start_is_after_a_terminator() {
    let s = "a\u{2028}b\rc";
    assert!(line_start(s, 0));
    assert!(!line_start(s, 1));
    assert!(line_start(s, 4));
    assert!(line_start(s, 6));
    assert!(!line_start(s, 2));
}

#[test]
fn chars_and_space_edges_read_unicode() {
    assert_eq!(char_at("a\u{e9}", 1), Some('\u{e9}'));
    assert_eq!(char_at("a\u{e9}", 2), None);
    assert_eq!(space_prefix("\u{3000} x \u{feff}"), 4);
    assert_eq!(space_suffix("\u{3000} x \u{feff}"), 4);
}
