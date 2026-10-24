use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use super::{Intensity, compress};

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
