//! Dispatch of `chertmem-dev <guard> [files...]` (dev §I): exit 0 hold, 1
//! finding, 2 usage or unreadable input.

#[cfg(test)]
mod tests;

use crate::name_guard;
use std::path::Path;

const USAGE: &str = "usage: chertmem-dev name-guard [files...]";

/// Run one guard from `args` against the repository at `root`.
#[must_use]
pub fn run(args: &[String], root: &Path) -> u8 {
    match args.split_first() {
        Some((verb, files)) if verb == "name-guard" => name(root, files),
        _ => {
            eprintln!("{USAGE}");
            2
        }
    }
}

fn name(root: &Path, files: &[String]) -> u8 {
    let Some(names) = denylist(root) else {
        return 2;
    };
    let mut texts = Vec::new();
    for f in files {
        let Ok(bytes) = std::fs::read(root.join(f)) else {
            eprintln!("name-guard: cannot read {f}");
            return 2;
        };
        texts.push((f.clone(), String::from_utf8_lossy(&bytes).into_owned()));
    }
    report(&name_guard::scan(&names, &texts))
}

/// The denylist's names; none when the file is absent (the CI half has
/// no denylist), `None` when it exists but cannot be read.
fn denylist(root: &Path) -> Option<Vec<String>> {
    match std::fs::read_to_string(root.join(".private/denylist")) {
        Ok(text) => Some(name_guard::names(&text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(Vec::new()),
        Err(_) => {
            eprintln!("name-guard: cannot read .private/denylist");
            None
        }
    }
}

fn report(found: &[name_guard::Finding]) -> u8 {
    for f in found {
        eprintln!(
            "{}:{}: holds a name from .private/denylist; reword it, or keep \
             the text out of the repository",
            f.path, f.line
        );
    }
    u8::from(!found.is_empty())
}
