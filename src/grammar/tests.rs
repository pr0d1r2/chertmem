use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

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

/// The corpus cases where chert's output differs from the golden file
/// because the last punctuation pass skips code (src/grammar V4, T5), as
/// (corpus file, intensity, chert's exact bytes). No corpus text has a
/// blank before `.,;:!?` inside code, so the list is empty and every
/// case must match the oracle; `CODE_DEVIATIONS` pins the deviation.
const DEVIATES: [(&str, &str, &str); 0] = [];

/// What `compress` must give for a corpus case: the golden file, or the
/// bytes `DEVIATES` lists for it.
fn expected(name: &str, level: &str) -> Result<String, Box<dyn Error>> {
    let listed = DEVIATES.iter().find(|d| d.0 == name && d.1 == level);
    match listed {
        Some(&(_, _, bytes)) => Ok(bytes.to_owned()),
        None => golden(name, level, "compressed"),
    }
}

#[test]
fn compress_matches_oracle_bytes() -> Res {
    for (name, text) in corpus()? {
        for (intensity, level) in INTENSITIES {
            let want = expected(&name, level)?;
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
/// V4, R1): its last pass drops blanks before `.,;:!?` inside code too.
/// Each is (text, intensity, cavemem's bytes, chert's bytes).
const CODE_DEVIATIONS: [(&str, Intensity, &str, &str); 6] = [
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
    // cavemem's tokenizer takes `` ` c ` `` (the fence's last tick) as
    // inline code, which then loses to the fence, so `d \t,` is prose:
    // only the fence keeps its blank.
    (
        "x ```b ?``` c `d \t,` e",
        Intensity::Full,
        "x ```b?``` c `d,` e",
        "x ```b ?``` c `d,` e",
    ),
];

#[test]
fn blank_before_a_mark_survives_inside_code() {
    for (text, intensity, _, want) in CODE_DEVIATIONS {
        assert_eq!(compress(text, intensity), want, "{text:?}");
    }
}

/// The deviation is real and is the pass alone: cavemem's bytes are
/// chert's with the pass run once more over the whole output, code too.
#[test]
fn code_deviations_are_the_pass_over_code_only() {
    for (text, _, cavemem, chert) in CODE_DEVIATIONS {
        assert_ne!(cavemem, chert, "{text:?}");
        assert_eq!(tighten(chert), cavemem, "{text:?}");
    }
}

/// Outside fenced and inline code the pass still runs (V4), also next
/// to and across kept spans that are not code (cavemem 0.2.1's bytes).
const PROSE_MARKS: [(&str, Intensity, &str); 4] = [
    ("x ,y \t; z !", Intensity::Full, "x,y; z!"),
    (
        "lives in ./config/a.toml now",
        Intensity::Full,
        "lives in./config/a.toml now",
    ),
    (
        "see https://x.y/z . ok",
        Intensity::Lite,
        "see https://x.y/z. ok",
    ),
    ("`a` . and ```b``` ;", Intensity::Full, "`a`. and ```b```;"),
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
            let want = golden(&name, level, "expanded")?;
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
