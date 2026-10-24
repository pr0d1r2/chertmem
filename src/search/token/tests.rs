use super::{terms, tokens};

#[test]
fn tokens_split_on_anything_not_alphanumeric_and_lowercase() {
    let got = tokens("Fix the cargo.lock -- Cargo  build_42!");
    let want = ["fix", "the", "cargo", "lock", "cargo", "build", "42"];
    assert_eq!(got, want);
}

#[test]
fn terms_are_distinct_and_sorted() {
    assert_eq!(terms("b a B c a"), ["a", "b", "c"]);
    assert!(terms(" -- ").is_empty());
}

#[test]
fn non_ascii_letters_are_tokens_too() {
    let upper = "\u{17b}\u{f3}\u{141}w pije";
    assert_eq!(tokens(upper), ["\u{17c}\u{f3}\u{142}w", "pije"]);
}
