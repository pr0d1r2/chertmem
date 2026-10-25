//! The release verbs: `notices [--check]` (dev T11). Every cargo call
//! goes through `external::stdout`, so a failed one is reported with its
//! stderr, never read as an empty crate list (rekall#47).

#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::path::Path;

use crate::external;
use crate::notices::{self, Entry, Meta, TARGETS};
use crate::spdx;

pub(super) const NOTICES: &str = "THIRD-PARTY-NOTICES.md";

type Crates = BTreeSet<(String, String)>;

/// The release verb `verb` with `rest`; `None` when it is not one.
pub(super) fn run(verb: &str, rest: &[String], root: &Path) -> Option<u8> {
    match (verb, rest) {
        ("notices", []) => Some(notices(root, "cargo", false)),
        ("notices", [flag]) if flag == "--check" => {
            Some(notices(root, "cargo", true))
        }
        _ => None,
    }
}

/// `notices [--check]` with `cargo` as the cargo binary: write the
/// page, or exit 1 when it differs.
pub(super) fn notices(root: &Path, cargo: &str, check: bool) -> u8 {
    let (old, page) = match page(root, cargo) {
        Ok(both) => both,
        Err(why) => return broken(&why),
    };
    if old == page {
        return 0;
    }
    if check {
        eprintln!(
            "notices: {NOTICES} is not the page Cargo.lock makes; run cargo \
             run -q -p chertmem-dev -- notices"
        );
        return 1;
    }
    write(root, &page)
}

fn broken(why: &str) -> u8 {
    eprintln!("notices: {why}");
    2
}

fn write(root: &Path, page: &str) -> u8 {
    std::fs::write(root.join(NOTICES), page).map_or_else(
        |e| broken(&format!("cannot write {NOTICES}: {e}")),
        |()| 0,
    )
}

/// The page on disk and the page the lock makes.
fn page(root: &Path, cargo: &str) -> Result<(String, String), String> {
    let old = std::fs::read_to_string(root.join(NOTICES))
        .map_err(|e| format!("cannot read {NOTICES}: {e}"))?;
    let entries = entries(root, cargo)?;
    let page = notices::splice(&old, &notices::render(&entries))?;
    Ok((old, page))
}

fn entries(root: &Path, cargo: &str) -> Result<Vec<Entry>, String> {
    let meta = notices::metadata(&external::stdout(root, cargo, &METADATA)?)?;
    let default = tree(root, cargo, &[])?;
    let bpe = tree(root, cargo, &["--features", "bpe"])?;
    let all: Crates = default.union(&bpe).cloned().collect();
    let mut out = Vec::new();
    for key in &all {
        let m = meta.get(key).ok_or_else(|| unknown(key))?;
        if m.registry {
            out.push(entry(key, m, !default.contains(key))?);
        }
    }
    Ok(out)
}

/// Every package, `bpe`'s too, with its license and source directory.
const METADATA: [&str; 6] = [
    "metadata",
    "--format-version",
    "1",
    "--all-features",
    "--locked",
    "--offline",
];

fn unknown((name, version): &(String, String)) -> String {
    format!("{name} {version} is in cargo tree but not in cargo metadata")
}

/// `cargo tree` of the package, 1 line per crate, offline.
const TREE: [&str; 11] = [
    "tree",
    "-p",
    "chertmem",
    "-e",
    "normal,build",
    "--prefix",
    "none",
    "--format",
    "{p}",
    "--locked",
    "--offline",
];

/// The crates `cargo tree` builds into `chert` on every release system.
fn tree(root: &Path, cargo: &str, extra: &[&str]) -> Result<Crates, String> {
    let mut args = TREE.to_vec();
    for t in TARGETS {
        args.extend(["--target", t]);
    }
    args.extend(extra);
    Ok(notices::shipped(&external::stdout(root, cargo, &args)?))
}

fn entry(
    (name, version): &(String, String),
    m: &Meta,
    bpe_only: bool,
) -> Result<Entry, String> {
    let license = match &m.license {
        Some(expr) => spdx::choose(expr)?,
        None => "see its texts".to_owned(),
    };
    Ok(Entry {
        name: name.clone(),
        version: version.clone(),
        texts: texts(&m.dir, &license)?,
        license,
        repository: m.repository.clone(),
        bpe_only,
    })
}

/// The cleaned license and notice texts in `dir` for `license`.
fn texts(dir: &Path, license: &str) -> Result<Vec<String>, String> {
    let shown = dir.display();
    let names: Vec<String> = std::fs::read_dir(dir)
        .map_err(|e| format!("cannot list {shown}: {e}"))?
        .filter_map(Result::ok)
        .filter(|d| d.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|d| d.file_name().into_string().ok())
        .collect();
    let read = |f: &String| {
        std::fs::read(dir.join(f))
            .map(|b| notices::clean(&String::from_utf8_lossy(&b)))
            .map_err(|e| format!("cannot read {shown}/{f}: {e}"))
    };
    notices::wanted(&names, license).iter().map(read).collect()
}
