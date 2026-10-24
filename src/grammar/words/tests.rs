use super::{match_case, replace};

fn upper(found: &str, _: usize) -> String {
    found.to_uppercase()
}

#[test]
fn keys_match_whole_words_ignoring_case() {
    assert_eq!(replace("the theme The", &["the"], upper), "THE theme THE");
}

#[test]
fn first_key_that_ends_at_a_boundary_wins() {
    let keys = ["w/o", "w/"];
    let with = |_: &str, k: usize| format!("<{k}>");
    assert_eq!(replace("w/o w/x w/ ", &keys, with), "<0> <1>x w/ ");
}

#[test]
fn no_keys_leave_text_as_is() {
    assert_eq!(
        replace("Z\u{fc}rich au lait", &[], upper),
        "Z\u{fc}rich au lait"
    );
}

#[test]
fn non_ascii_never_matches_an_ascii_key() {
    assert_eq!(replace("\u{17f}at sat", &["sat"], upper), "\u{17f}at SAT");
}

#[test]
fn case_follows_the_found_word() {
    assert_eq!(match_case("CFG", "configuration"), "CONFIGURATION");
    assert_eq!(match_case("Cfg", "configuration"), "Configuration");
    assert_eq!(match_case("cfg", "configuration"), "configuration");
    assert_eq!(match_case("B/C", "because"), "BECAUSE");
    assert_eq!(match_case("And", "&"), "&");
}
