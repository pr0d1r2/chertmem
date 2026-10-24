//! Each expectation is cavemem 0.2.1's own `tokenize` output for the
//! same input (src/grammar V1), as `(text, preserved)` pairs.

use super::tokenize;

fn segs(text: &str) -> Vec<(&str, bool)> {
    tokenize(text).iter().map(|s| (s.text, s.kept)).collect()
}

#[test]
fn fence_is_the_shortest_closed_one() {
    let want = [("a ", false), ("```x```", true), (" b ``` c", false)];
    assert_eq!(segs("a ```x``` b ``` c"), want);
}

#[test]
fn inline_code_stays_on_one_line() {
    let want = [
        ("x ", false),
        ("`a`", true),
        (" y `", false),
        ("` `", true),
        ("b\nc`", false),
    ];
    assert_eq!(segs("x `a` y `` `b\nc`"), want);
}

#[test]
fn urls_stop_at_brackets_and_need_a_boundary() {
    let want = [
        ("see ", false),
        ("https://a.b/c", true),
        (") and (", false),
        ("http://x", true),
        ("] httpx:", false),
        ("//y", true),
        (" ahttp:", false),
        ("//z", true),
    ];
    let text = "see https://a.b/c) and (http://x] httpx://y ahttp://z";
    assert_eq!(segs(text), want);
}

#[test]
fn headings_start_lines_and_take_one_to_six_marks() {
    let want = [
        ("#\nfoo", true),
        ("\n", false),
        ("## bar", true),
        ("\n####### no\n#x\r", false),
        ("# cr", true),
    ];
    assert_eq!(segs("#\nfoo\n## bar\n####### no\n#x\r# cr"), want);
}

#[test]
fn paths_are_relative_home_or_windows() {
    let want = [
        ("a ", false),
        ("../b", true),
        (" ", false),
        ("./c", true),
        (" .", false),
        ("../d", true),
        (" ", false),
        ("~/e", true),
        (" ", false),
        ("C:\\f\\g", true),
        (" / ./ c:\\x", false),
    ];
    assert_eq!(segs("a ../b ./c .../d ~/e C:\\f\\g / ./ c:\\x"), want);
}

#[test]
fn dates_back_off_to_a_boundary() {
    let want = [
        ("2024-01-02T03:04", true),
        (":05x ", false),
        ("2024-01-02 03:04", true),
        (" ", false),
        ("2024", true),
        ("-", false),
        ("01", true),
        ("-02x ", false),
        ("2024-01-02T03:04:05", true),
    ];
    let text = "2024-01-02T03:04:05x 2024-01-02 03:04 2024-01-02x \
                2024-01-02T03:04:05";
    assert_eq!(segs(text), want);
}

const VERSIONS: [(&str, bool); 12] = [
    ("v1.2", true),
    (" ", false),
    ("1.2.3-rc.1", true),
    (". ", false),
    ("1.2.3", true),
    ("-. ", false),
    ("1.2+x_y", true),
    (" ", false),
    ("1", true),
    (".2a xv1.", false),
    ("2", true),
    (" v1", false),
];

#[test]
fn versions_take_the_longest_suffix_at_a_boundary() {
    let text = "v1.2 1.2.3-rc.1. 1.2.3-. 1.2+x_y 1.2a xv1.2 v1";
    assert_eq!(segs(text), VERSIONS);
}

#[test]
fn numbers_need_a_boundary_on_both_sides() {
    let want = [
        ("3.14", true),
        (" 42x ", false),
        ("7", true),
        (". _9 ", false),
        ("1", true),
        (".x", false),
    ];
    assert_eq!(segs("3.14 42x 7. _9 1.x"), want);
}

#[test]
fn identifiers_are_snake_kebab_or_camel() {
    let want = [
        ("foo_bar", true),
        (" ", false),
        ("a-b", true),
        ("- ", false),
        ("getX", true),
        (" _x_ ", false),
        ("x_-y", true),
        (" aB_ ab- getX_ ", false),
        ("a__", true),
    ];
    let text = "foo_bar a-b- getX _x_ x_-y aB_ ab- getX_ a__";
    assert_eq!(segs(text), want);
}

#[test]
fn unicode_space_counts_as_js_white_space() {
    let want = [
        ("Z\u{fc}rich ", false),
        ("2024-01-02\u{3000}03:04", true),
        (" #\u{3000}title", false),
    ];
    let text = "Z\u{fc}rich 2024-01-02\u{3000}03:04 #\u{3000}title";
    assert_eq!(segs(text), want);
    assert_eq!(segs("#\u{3000}title"), [("#\u{3000}title", true)]);
}

#[test]
fn empty_text_has_no_segments() {
    assert!(segs("").is_empty());
}

/// Each kept span of `CODE_TEXT` and whether it is code.
const CODE_SPANS: [(&str, bool); 8] = [
    ("`g`", true),
    ("```f```", true),
    ("~~~h~~~", true),
    ("/p", false),
    ("https://u", false),
    ("v1.2.3", false),
    ("x_y", false),
    ("7", false),
];

const CODE_TEXT: &str = "a `g` ```f``` ~~~h~~~ /p https://u v1.2.3 x_y 7";

/// Only fences and inline code count as code (src/grammar V4); the
/// expectation is chert's own, cavemem has no such flag.
#[test]
fn only_fences_and_inline_code_are_code() {
    let code: Vec<(&str, bool)> = tokenize(CODE_TEXT)
        .iter()
        .filter(|s| s.kept)
        .map(|s| (s.text, s.code))
        .collect();
    assert_eq!(code, CODE_SPANS);
}

/// Only paths count as paths (src/grammar V5 (c)); chert's own flag.
#[test]
fn only_paths_are_paths() {
    let paths: Vec<&str> = tokenize(CODE_TEXT)
        .iter()
        .filter(|s| s.path)
        .map(|s| s.text)
        .collect();
    assert_eq!(paths, ["/p"]);
}

/// Inline code right after a fence is code (src/grammar V5 (b)); cavemem
/// takes `` ` ` `` from the fence's last tick, so `c` is prose there.
#[test]
fn inline_code_after_a_fence_is_code() {
    let want = [
        ("x ", false),
        ("```b```", true),
        (" ", false),
        ("`c`", true),
        (" ", false),
        ("`d`", true),
    ];
    assert_eq!(segs("x ```b``` `c` `d`"), want);
}

/// Inline code after a fence that runs into a later fence wins over it,
/// as after prose; cavemem keeps `` ```the c``` `` as the fence.
#[test]
fn inline_code_after_a_fence_beats_a_later_fence() {
    let want = [
        ("```a```", true),
        (" ", false),
        ("`b `", true),
        ("`", false),
        ("`the c`", true),
        ("``", false),
    ];
    assert_eq!(segs("```a``` `b ```the c```"), want);
}

/// A fence that lost to earlier inline code cuts no inline code: the
/// segments stay cavemem's.
#[test]
fn a_lost_fence_cuts_no_inline_code() {
    let want = [
        ("`a `", true),
        ("`", false),
        ("`b`", true),
        ("`", false),
        ("`c`", true),
    ];
    assert_eq!(segs("`a ```b```c`"), want);
}
