use super::strip;

#[test]
fn a_closed_span_is_removed_in_any_case() {
    assert_eq!(strip("a <private>k1</private> b"), "a  b");
    assert_eq!(strip("<PRIVATE>k</Private>x<private>y</private>"), "x");
}

#[test]
fn an_unclosed_span_drops_the_rest() {
    assert_eq!(strip("keep <private>secret"), "keep ");
    assert_eq!(strip("a</private>b"), "a</private>b");
}

#[test]
fn text_without_spans_is_kept() {
    assert_eq!(strip("plain \u{e9}t\u{e9}"), "plain \u{e9}t\u{e9}");
    assert_eq!(strip(""), "");
}
