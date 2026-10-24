mod articles;
mod deviates;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use articles::{ARTICLE_DEVIATIONS, ARTICLE_PROSE};
use deviates::DEVIATES;

use super::prose::tighten;
use super::{Intensity, compress, expand};

type Res = Result<(), Box<dyn Error>>;

const INTENSITIES: [(Intensity, &str); 3] = [
    (Intensity::Lite, "lite"),
    (Intensity::Full, "full"),
    (Intensity::Ultra, "ultra"),
];

fn dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/grammar")
        .join(name)
}

/// Every corpus text, as its name and contents, sorted by name.
fn corpus() -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir("corpus"))? {
        let path = entry?.path();
        let stem = path.file_stem().and_then(|s| s.to_str());
        let name = stem.ok_or("corpus file without a name")?.to_owned();
        out.push((name, fs::read_to_string(&path)?));
    }
    out.sort();
    Ok(out)
}

fn golden(
    name: &str,
    level: &str,
    kind: &str,
) -> Result<String, Box<dyn Error>> {
    let file = dir("oracle").join(format!("{name}.{level}.{kind}"));
    Ok(fs::read_to_string(file)?)
}

#[test]
fn corpus_is_not_empty() -> Res {
    assert!(corpus()?.len() >= 8);
    Ok(())
}

/// What chert must give for a corpus case and golden kind: the golden
/// file, or the bytes `DEVIATES` lists for it.
fn expected(
    name: &str,
    level: &str,
    kind: &str,
) -> Result<String, Box<dyn Error>> {
    let listed = DEVIATES
        .iter()
        .find(|d| d.0 == name && d.1 == level && d.2 == kind);
    match listed {
        Some(&(_, _, _, bytes)) => Ok(bytes.to_owned()),
        None => golden(name, level, kind),
    }
}

/// Each listed case really differs from its golden file, and only by
/// the blanks before the two paths (deviation (c)).
#[test]
fn deviates_differ_from_golden_only_before_paths() -> Res {
    for (name, level, kind, bytes) in DEVIATES {
        let gold = golden(name, level, kind)?;
        assert_ne!(gold, bytes, "{name}.{level}.{kind}");
        let undone = bytes
            .replace(" ./config", "./config")
            .replace(" ../shared", "../shared");
        assert_eq!(undone, gold, "{name}.{level}.{kind}");
    }
    Ok(())
}

#[test]
fn compress_matches_oracle_bytes() -> Res {
    for (name, text) in corpus()? {
        for (intensity, level) in INTENSITIES {
            let want = expected(&name, level, "compressed")?;
            assert_eq!(compress(&text, intensity), want, "{name}.{level}");
        }
    }
    Ok(())
}

#[test]
fn off_is_identity() -> Res {
    for (_, text) in corpus()? {
        assert_eq!(compress(&text, Intensity::Off), text);
    }
    Ok(())
}

#[test]
fn compress_is_deterministic() -> Res {
    for (_, text) in corpus()? {
        for (intensity, _) in INTENSITIES {
            assert_eq!(compress(&text, intensity), compress(&text, intensity));
        }
    }
    Ok(())
}

/// Spans the grammar must keep, written out by hand from the corpus.
const KEPT: [&str; 16] = [
    "```rust\nfn load_configuration(path: &Path) -> Result<Config, Error> {",
    "~~~\nthe very same text, just kept as it is\n~~~",
    "`cargo test --workspace`",
    "https://example.com/docs/the-guide?section=a",
    "http://mirror.example.org/repository/(main",
    "./config/chert.toml",
    "/usr/local/bin/chert",
    "~/.config/chert/settings.toml",
    "C:\\Program\\chert.exe",
    "1.2.0-beta.1",
    "0.2.1+build.7",
    "2024-03-02T10:30:45",
    "load_config",
    "readFileSync",
    "kebab-case-name",
    "# The Configuration of the Repository",
];

#[test]
fn protected_spans_survive_every_intensity() -> Res {
    let all: String = corpus()?.into_iter().map(|(_, t)| t).collect();
    for (intensity, level) in INTENSITIES {
        let out = compress(&all, intensity);
        for span in KEPT {
            assert!(all.contains(span), "{span:?} not in the corpus");
            assert!(out.contains(span), "{level}: {span:?} was touched");
        }
    }
    Ok(())
}

/// Edge cases outside the corpus, each with cavemem 0.2.1's own output.
const EDGES: [(&str, Intensity, &str); 12] = [
    (
        "  the  thing ,  \n  \u{a0} is\r\nthe end . \u{2028} the x",
        Intensity::Full,
        " thing,\n\u{a0} is\r\nend.\u{2028}x",
    ),
    ("The THE tHe a An x", Intensity::Full, " tHe x"),
    (
        "AND And and with w/o b4 x",
        Intensity::Ultra,
        "& & & w/ w/o b4 x",
    ),
    ("\t\tthe\t\tend\t\t", Intensity::Full, " end "),
    ("   ", Intensity::Full, "   "),
    ("", Intensity::Full, ""),
    (
        "Configuration CONFIGURATION configuration",
        Intensity::Ultra,
        "Cfg CFG cfg",
    ),
    (
        "I think that it is fine. i think so",
        Intensity::Ultra,
        "that it is fine. so",
    ),
    (
        "line one  \r\n  line two\n\n\n\nend",
        Intensity::Lite,
        "line one\r\nline two\n\nend",
    ),
    ("a  \u{2029}  b", Intensity::Full, "\u{2029}b"),
    ("the\n", Intensity::Full, "\n"),
    (" x \n", Intensity::Full, " x\n"),
];

#[test]
fn edge_cases_match_oracle_bytes() {
    for (text, intensity, want) in EDGES {
        assert_eq!(compress(text, intensity), want, "{text:?}");
    }
}

/// Inputs where chert deliberately differs from cavemem 0.2.1 (src/grammar
/// V5 (a), (b), R1, R4): its last pass drops blanks before `.,;:!?`
/// inside code too, and it loses inline code right after a fence to the
/// fence's last tick. Each is (text, intensity, cavemem's bytes, chert's).
const CODE_DEVIATIONS: [(&str, Intensity, &str, &str); 9] = [
    (
        "`a .b` and x ;",
        Intensity::Full,
        "`a.b` and x;",
        "`a .b` and x;",
    ),
    (
        "```\nlet x = a .b ;\n``` the end .",
        Intensity::Full,
        "```\nlet x = a.b;\n``` end.",
        "```\nlet x = a .b ;\n``` end.",
    ),
    (
        "~~~\nf(a , b)\t!\n~~~ x",
        Intensity::Lite,
        "~~~\nf(a, b)!\n~~~ x",
        "~~~\nf(a , b)\t!\n~~~ x",
    ),
    (
        "run `x ; y` then `p :q` .",
        Intensity::Full,
        "run `x; y` then `p:q`.",
        "run `x ; y` then `p :q`.",
    ),
    (
        "x `d \t,` c ```b ?``` e",
        Intensity::Full,
        "x `d,` c ```b?``` e",
        "x `d \t,` c ```b ?``` e",
    ),
    // cavemem takes `` ` c ` `` (the fence's last tick) as inline code,
    // which loses to the fence, so `d \t,` is prose there (R4); chert
    // keeps it as code (V5 (b)).
    (
        "x ```b ?``` c `d \t,` e",
        Intensity::Full,
        "x ```b?``` c `d,` e",
        "x ```b ?``` c `d \t,` e",
    ),
    (
        "x ```b``` `c ;` `d ,`",
        Intensity::Full,
        "x ```b``` `c;` `d,`",
        "x ```b``` `c ;` `d ,`",
    ),
    (
        "```a```\n`b ;` the x",
        Intensity::Full,
        "```a```\n`b;` x",
        "```a```\n`b ;` x",
    ),
    (
        "~~~a~~~ `b .` c",
        Intensity::Lite,
        "~~~a~~~ `b.` c",
        "~~~a~~~ `b .` c",
    ),
];

/// Inputs where chert keeps the blank before a path that starts with a
/// mark (`./`, `../`) and cavemem drops it (src/grammar V5 (c)). Only
/// paths: no other kept span can start with `.,;:!?`. A blank before a
/// mark after a path, or before `.` of `.../x` (not a path), still goes.
/// Each is (text, intensity, cavemem's bytes, chert's bytes).
const PATH_DEVIATIONS: [(&str, Intensity, &str, &str); 4] = [
    (
        "lives in ./config/a.toml .",
        Intensity::Full,
        "lives in./config/a.toml.",
        "lives in ./config/a.toml.",
    ),
    (
        "go up ../a , then ~/b and C:\\x ; v1.2 .",
        Intensity::Full,
        "go up../a, then ~/b and C:\\x; v1.2.",
        "go up ../a, then ~/b and C:\\x; v1.2.",
    ),
    (
        "the parent is \t../shared/lib .",
        Intensity::Lite,
        "the parent is../shared/lib.",
        "the parent is ../shared/lib.",
    ),
    (
        "see .../x and a ./b",
        Intensity::Full,
        "see.../x and./b",
        "see.../x and ./b",
    ),
];

#[test]
fn blank_before_a_mark_survives_inside_code() {
    for (text, intensity, _, want) in CODE_DEVIATIONS {
        assert_eq!(compress(text, intensity), want, "{text:?}");
    }
}

#[test]
fn blank_before_a_path_survives() {
    for (text, intensity, _, want) in PATH_DEVIATIONS {
        assert_eq!(compress(text, intensity), want, "{text:?}");
    }
}

/// The deviations are real and are the pass alone: cavemem's bytes are
/// chert's with the pass run once more over the whole output, code and
/// the blank before a path too.
#[test]
fn deviations_are_the_pass_over_code_and_paths_only() {
    for (text, _, cavemem, chert) in
        CODE_DEVIATIONS.iter().chain(&PATH_DEVIATIONS)
    {
        assert_ne!(cavemem, chert, "{text:?}");
        assert_eq!(tighten(chert), *cavemem, "{text:?}");
    }
}

/// Inline code right after a fence is code to `expand` too: cavemem
/// drops `the` in it and writes `cfg` out again, chert keeps both.
#[test]
fn inline_code_after_a_fence_survives_expand() {
    let text = "x ```b``` `the cfg` d";
    let out = compress(text, Intensity::Full);
    assert_eq!(out, text, "cavemem: x ```b``` ` cfg` d");
    assert_eq!(expand(&out), text, "cavemem: x ```b``` ` configuration` d");
}

/// Outside fenced and inline code the pass still runs (V5), also next
/// to and across kept spans that are not code (cavemem 0.2.1's bytes),
/// except the blank before a path (V5 (c)).
const PROSE_MARKS: [(&str, Intensity, &str); 5] = [
    ("x ,y \t; z !", Intensity::Full, "x,y; z!"),
    (
        "lives in ./config/a.toml now",
        Intensity::Full,
        "lives in ./config/a.toml now",
    ),
    (
        "see https://x.y/z . ok",
        Intensity::Lite,
        "see https://x.y/z. ok",
    ),
    ("`a` . and ```b``` ;", Intensity::Full, "`a`. and ```b```;"),
    (
        "v1.2 . and x_y , see 2024-01-02 ; https://a/b ! # h ?",
        Intensity::Full,
        "v1.2. and x_y, see 2024-01-02; https://a/b! # h?",
    ),
];

#[test]
fn blank_before_a_mark_still_goes_outside_code() {
    for (text, intensity, want) in PROSE_MARKS {
        assert_eq!(compress(text, intensity), want, "{text:?}");
    }
}

#[test]
fn expand_of_compress_matches_oracle_bytes() -> Res {
    for (name, text) in corpus()? {
        for (intensity, level) in INTENSITIES {
            let want = expected(&name, level, "expanded")?;
            let got = expand(&compress(&text, intensity));
            assert_eq!(got, want, "{name}.{level}");
        }
    }
    Ok(())
}

/// Expand inputs outside the corpus, each with cavemem 0.2.1's output.
const EXPANDS: [(&str, &str); 4] = [
    (
        "w/foo w/ w/o b/c B4 CFG Cfg cfg fn() dbs",
        "w/foo w/ w/o b/c BEFORE CONFIGURATION Configuration configuration \
         function() databases",
    ),
    (
        "the repo/x REPO Repo rEPO",
        "the repository/x REPOSITORY Repository rEPO",
    ),
    (
        "`cfg` cfg https://x/cfg cfg_name",
        "`cfg` configuration https://x/cfg cfg_name",
    ),
    ("", ""),
];

#[test]
fn expand_edge_cases_match_oracle_bytes() {
    for (text, want) in EXPANDS {
        assert_eq!(expand(text), want, "{text:?}");
    }
}

/// Deviation (d) of V5 (B1): `articles` lists the inputs.
#[test]
fn a_file_name_that_starts_with_an_article_stays_whole() {
    for (text, intensity, cavemem, chert) in ARTICLE_DEVIATIONS {
        assert_ne!(cavemem, chert, "{text:?}");
        assert_eq!(compress(text, intensity), chert, "{text:?}");
    }
}

#[test]
fn an_article_that_starts_no_file_name_still_goes() {
    for (text, intensity, want) in ARTICLE_PROSE {
        assert_eq!(compress(text, intensity), want, "{text:?}");
    }
}

/// `expand` keeps the name too: cavemem gives `cfg.cfg` at ultra and
/// expands it to `configuration.configuration`.
#[test]
fn a_file_name_that_starts_with_an_article_survives_expand() {
    let out = compress("the cfg a.cfg", Intensity::Ultra);
    assert_eq!(out, "cfg a.cfg");
    assert_eq!(expand(&out), "configuration a.cfg");
}
