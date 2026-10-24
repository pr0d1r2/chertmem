use super::{article_path, fence, heading, inline, path, url};

#[test]
fn fences_close_with_their_own_mark() {
    assert_eq!(fence("```a~~~b```", 0), Some(11));
    assert_eq!(fence("~~~a```b~~~ ~~~", 0), Some(11));
    assert_eq!(fence("``` open", 0), None);
    assert_eq!(fence("``````", 0), Some(6));
    assert_eq!(fence("x", 5), None);
}

#[test]
fn inline_code_needs_one_char_and_one_line() {
    assert_eq!(inline("`a` b", 0), Some(3));
    assert_eq!(inline("``", 0), None);
    assert_eq!(inline("`a\nb`", 0), None);
    assert_eq!(inline("`a", 0), None);
}

#[test]
fn urls_need_a_scheme_and_a_tail() {
    assert_eq!(url("https://a b", 0), Some(9));
    assert_eq!(url("http://a\u{3000}b", 0), Some(8));
    assert_eq!(url("http:// x", 0), None);
    assert_eq!(url("ftp://a", 0), None);
    assert_eq!(url("xhttp://a", 1), None);
}

#[test]
fn headings_take_the_rest_of_the_line() {
    assert_eq!(heading("## a b\nc", 0), Some(6));
    assert_eq!(heading("###### six", 0), Some(10));
    assert_eq!(heading("####### seven", 0), None);
    assert_eq!(heading("#x", 0), None);
    assert_eq!(heading("a # b", 2), None);
}

/// Paths tried at byte 0, with the match end the regex gives.
const PATHS: [(&str, Option<usize>); 9] = [
    ("/usr/bin x", Some(8)),
    ("../a", Some(4)),
    (".../a", None),
    ("~/a", Some(3)),
    ("~a", None),
    ("C:\\a\\b c", Some(6)),
    ("c:\\a", None),
    ("C:/a", None),
    ("/ a", None),
];

#[test]
fn paths_take_their_class() {
    for (text, want) in PATHS {
        assert_eq!(path(text, 0), want, "{text:?}");
    }
}

/// Article paths tried at byte 0, with chert's match end (V5 (d)).
const ARTICLE_PATHS: [(&str, Option<usize>); 9] = [
    ("a/b.ipynb x", Some(9)),
    ("an.txt.", Some(7)),
    ("THE.md", Some(6)),
    ("a//", Some(3)),
    ("a/ b", None),
    ("a. b", None),
    ("a.", None),
    ("ab.txt", None),
    ("a-b.txt", None),
];

#[test]
fn article_paths_start_a_name() {
    for (text, want) in ARTICLE_PATHS {
        assert_eq!(article_path(text, 0), want, "{text:?}");
    }
    assert_eq!(article_path("xa.txt", 1), None);
}
