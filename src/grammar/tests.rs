use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

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

#[test]
fn compress_matches_oracle_bytes() -> Res {
    for (name, text) in corpus()? {
        for (intensity, level) in INTENSITIES {
            let want = golden(&name, level, "compressed")?;
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

/// cavemem's last pass drops blanks before `.,;:!?` across the whole
/// output, kept spans included (src/grammar R1): V1 wins over V2 here.
#[test]
fn blank_before_a_mark_leaks_into_kept_code() {
    assert_eq!(compress("`a .b` and x ;", Intensity::Full), "`a.b` and x;");
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
