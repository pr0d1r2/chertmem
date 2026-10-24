//! Dispatch of `chertmem-dev <guard> [files...]` (dev §I): exit 0 hold, 1
//! finding, 2 usage or unreadable input.

#[cfg(test)]
mod tests;

use crate::{lock_graph, mirror, name_guard, worktree};
use std::path::Path;

const USAGE: &str = "usage: chertmem-dev name-guard [files...] | \
                     no-core-worktree | lock-graph | rust-mirror";

/// Run one guard from `args` against the repository at `root`.
#[must_use]
pub fn run(args: &[String], root: &Path) -> u8 {
    match args.split_first() {
        Some((verb, files)) if verb == "name-guard" => name(root, files),
        Some((verb, [])) if verb == "no-core-worktree" => shared_config(root),
        Some((verb, [])) if verb == "lock-graph" => lock(root),
        Some((verb, [])) if verb == "rust-mirror" => mirror(root),
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

/// `no-core-worktree`: read the shared config through git's own idea of
/// the common directory, so a linked worktree checks the shared file.
fn shared_config(root: &Path) -> u8 {
    let text = match shared_config_text(root) {
        Ok(text) => text,
        Err(code) => return code,
    };
    let Some(path) = worktree::core_worktree(&text) else {
        return 0;
    };
    eprintln!(
        "core.worktree = {path} in the shared git config; run `git config \
         --unset core.worktree`"
    );
    1
}

/// The shared config's text, or exit code 2 when the repository or the
/// file cannot be found or read.
fn shared_config_text(root: &Path) -> Result<String, u8> {
    let Some(dir) = git_common_dir(root) else {
        eprintln!("no-core-worktree: not a git repository");
        return Err(2);
    };
    std::fs::read_to_string(dir.join("config")).map_err(|_| {
        eprintln!("no-core-worktree: cannot read the shared git config");
        2
    })
}

/// `git rev-parse --git-common-dir` in `root`.
fn git_common_dir(root: &Path) -> Option<std::path::PathBuf> {
    let rel = git(root, &["rev-parse", "--git-common-dir"])?;
    Some(root.join(rel.trim()))
}

/// `git <args>` in `root`, its stdout when it succeeds. The hook's
/// `GIT_*` variables are dropped, so only `root` decides the repository
/// (`paradigm/V15`).
fn git(root: &Path, args: &[&str]) -> Option<String> {
    let mut cmd = std::process::Command::new("git");
    for (k, _) in std::env::vars_os() {
        if k.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(k);
        }
    }
    let out = cmd.current_dir(root).args(args).output().ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    out.status.success().then_some(text)
}

/// `rust-mirror`: every tracked `.rs` file, read from the work tree.
fn mirror(root: &Path) -> u8 {
    let (Some(files), Some(manifests)) = (
        git(root, &["ls-files", "*.rs"]),
        git(root, &["ls-files", "Cargo.toml", "*/Cargo.toml"]),
    ) else {
        eprintln!("rust-mirror: not a git work tree; nothing was checked");
        return 2;
    };
    let crates: Vec<String> = manifests
        .lines()
        .map(|m| m.strip_suffix("Cargo.toml").unwrap_or(m).to_owned())
        .collect();
    let rs = read_tree(root, &files);
    report_lines("rust-mirror", &mirror::findings(&rs, &crates))
}

/// The work-tree text of each listed path. A tracked file deleted from
/// the work tree reads as empty, so it defines no logic.
fn read_tree(
    root: &Path,
    files: &str,
) -> std::collections::BTreeMap<String, String> {
    files
        .lines()
        .map(|f| {
            let text =
                std::fs::read_to_string(root.join(f)).unwrap_or_default();
            (f.to_owned(), text)
        })
        .collect()
}

/// Print each finding under the guard's name; exit 1 when there is one.
fn report_lines(guard: &str, found: &[String]) -> u8 {
    for f in found {
        eprintln!("{guard}: {f}");
    }
    u8::from(!found.is_empty())
}

/// `lock-graph`: the root flake.lock holds one nixpkgs and one pin node.
fn lock(root: &Path) -> u8 {
    let Ok(text) = std::fs::read_to_string(root.join("flake.lock")) else {
        eprintln!("lock-graph: cannot read flake.lock");
        return 2;
    };
    let found = lock_graph::findings(&text);
    for f in &found {
        eprintln!(
            "flake.lock: {f}; make the input follow nixpkgs-lock, then run \
             `nix flake lock`"
        );
    }
    u8::from(!found.is_empty())
}
