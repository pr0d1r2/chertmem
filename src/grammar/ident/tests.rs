use super::identifier;

/// Inputs tried at byte 0, with the match end the regex gives.
const AT_START: [(&str, Option<usize>); 13] = [
    ("foo_bar x", Some(7)),
    ("a-b-", Some(3)),
    ("x_-y", Some(4)),
    ("a__", Some(3)),
    ("_x_ ", None),
    ("ab- ", None),
    ("9_a", None),
    ("getX y", Some(4)),
    ("getHTTPResponse", Some(15)),
    ("aB_", None),
    ("getX_ ", None),
    ("Getx", None),
    ("get", None),
];

#[test]
fn snake_kebab_and_camel_case_match_as_the_regex() {
    for (text, want) in AT_START {
        assert_eq!(identifier(text, 0), want, "{text:?}");
    }
}

#[test]
fn identifiers_start_at_a_boundary() {
    assert_eq!(identifier("xfoo_bar", 1), None);
    assert_eq!(identifier("", 0), None);
}
