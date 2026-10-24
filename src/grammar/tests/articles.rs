//! Deviation (d) of src/grammar V5 (B1, T7): a path or file name whose
//! first part is an article word stays whole.

use crate::grammar::Intensity;

/// Inputs where cavemem 0.2.1's article filter splits `a`, `an` or
/// `the` off a path or file name and chert keeps it: the word at a `\b`,
/// any case, then `/` and a path byte or `.` and a letter or digit, as
/// one span over the `[A-Za-z0-9._\-/]` run. `pick a.The end` has no
/// blank after the dot, so it reads as a file name too. Each is (text,
/// intensity, cavemem's bytes, chert's bytes).
pub(super) const ARTICLE_DEVIATIONS: [(&str, Intensity, &str, &str); 11] = [
    ("cat a.txt", Intensity::Full, "cat.txt", "cat a.txt"),
    ("cat a.txt", Intensity::Ultra, "cat.txt", "cat a.txt"),
    ("cat the.txt", Intensity::Full, "cat.txt", "cat the.txt"),
    (
        "open a/b.ipynb now",
        Intensity::Full,
        "open/b.ipynb now",
        "open a/b.ipynb now",
    ),
    (
        "The.md and An.rs , A/b/c.py .",
        Intensity::Full,
        ".md and.rs,/b/c.py.",
        "The.md and An.rs, A/b/c.py.",
    ),
    (
        "cat a.txt ; the.md .",
        Intensity::Ultra,
        "cat.txt;.md.",
        "cat a.txt; the.md.",
    ),
    ("a.b/c the./x", Intensity::Full, ".b/c./x", "a.b/c ./x"),
    (
        "see x.a.txt and the.tar.gz",
        Intensity::Full,
        "see x..txt and.tar.gz",
        "see x.a.txt and the.tar.gz",
    ),
    ("a.k.a. x", Intensity::Full, ".k.. x", "a.k.a. x"),
    (
        "at 9 a.m. today",
        Intensity::Full,
        "at 9.m. today",
        "at 9 a.m. today",
    ),
    (
        "pick a.The end",
        Intensity::Full,
        "pick. end",
        "pick a.The end",
    ),
];

/// Article words that start no file name keep cavemem 0.2.1's bytes:
/// `a.` before a blank, a line end or another mark, `a/` with no path
/// byte after it, and `a` joined by `-` or `_` (an identifier).
pub(super) const ARTICLE_PROSE: [(&str, Intensity, &str); 6] = [
    (
        "I want a. Then the end.",
        Intensity::Full,
        "I want. Then end.",
    ),
    ("a.. b an.\nthe.", Intensity::Full, ".. b.\n."),
    ("a/ b the/", Intensity::Full, "/ b /"),
    ("a-b.txt a_b.txt", Intensity::Full, "a-b.txt a_b.txt"),
    ("data.txt", Intensity::Full, "data.txt"),
    ("cat a.txt", Intensity::Lite, "cat a.txt"),
];
