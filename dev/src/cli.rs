//! Dispatch of `chertmem-dev <guard> [files...]` (dev §I): exit 0 hold, 1
//! finding, 2 usage or unreadable input.

#[cfg(test)]
mod tests;

use crate::{name_guard, worktree};
use std::path::Path;

const USAGE: &str =
    "usage: chertmem-dev name-guard [files...] | no-core-worktree";

/// Run one guard from `args` against the repository at `root`.
#[must_use]
pub fn run(args: &[String], root: &Path) -> u8 {
    match args.split_first() {
        Some((verb, files)) if verb == "name-guard" => name(root, files),
        Some((verb, [])) if verb == "no-core-worktree" => shared_config(root),
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

/// `git rev-parse --git-common-dir` in `root`, with the hook's `GIT_*`
/// variables dropped, so only `root` decides the repository.
fn git_common_dir(root: &Path) -> Option<std::path::PathBuf> {
    let mut cmd = std::process::Command::new("git");
    for (k, _) in std::env::vars_os() {
        if k.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(k);
        }
    }
    let out = cmd
        .current_dir(root)
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .ok()?;
    let rel = String::from_utf8(out.stdout).ok()?;
    out.status.success().then(|| root.join(rel.trim()))
}
