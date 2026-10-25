//! Dispatch of `chertmem-dev <guard> [files...]` (dev §I): exit 0 hold, 1
//! finding, 2 usage or unreadable input.

#[cfg(test)]
mod tests;

mod bench;

use crate::{lock_graph, message, mirror, name_guard, tdd_order, worktree};
use std::path::Path;

const USAGE: &str = "usage: chertmem-dev name-guard [files...] | \
                     no-core-worktree | lock-graph | rust-mirror | \
                     tdd-order [RANGE] | commit-msg <file> | \
                     bench-record | bench-gate [fixture...] | \
                     perf-doc [--check]";

/// Run one guard from `args` against the repository at `root`.
#[must_use]
pub fn run(args: &[String], root: &Path) -> u8 {
    let Some((verb, rest)) = args.split_first() else {
        return usage();
    };
    match (verb.as_str(), rest) {
        ("name-guard", files) => name(root, files),
        ("no-core-worktree", []) => shared_config(root),
        ("lock-graph", []) => lock(root),
        ("rust-mirror", []) => mirror(root),
        ("tdd-order", []) => order(root, None),
        ("tdd-order", [range]) => order(root, Some(range)),
        ("commit-msg", [file]) => commit_msg(root, file),
        (verb, rest) => bench::run(verb, rest, root).unwrap_or_else(usage),
    }
}

/// `commit-msg <file>`: the message git hands the hook (dev V11).
fn commit_msg(root: &Path, file: &str) -> u8 {
    let Ok(bytes) = std::fs::read(root.join(file)) else {
        eprintln!("commit-msg: cannot read {file}");
        return 2;
    };
    let text = String::from_utf8_lossy(&bytes);
    report_lines("commit-msg", &message::findings(&text))
}

fn usage() -> u8 {
    eprintln!("{USAGE}");
    2
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

/// `tdd-order [RANGE]`: the range a push would publish by default
/// (`@{upstream}..HEAD`, or the whole history before the first push).
fn order(root: &Path, range: Option<&String>) -> u8 {
    let ex = match exempt_list(root) {
        Ok(ex) => ex,
        Err(why) => return report_lines("tdd-order", &[why]),
    };
    let range = range.cloned().unwrap_or_else(|| default_range(root));
    let Some(commits) = commits(root, &range) else {
        eprintln!("tdd-order: cannot resolve the range {range}");
        return 2;
    };
    report_lines("tdd-order", &tdd_order::findings(&commits, &ex))
}

/// `dev/tdd-order.exempt`; missing or malformed is a finding, since an
/// exemption nobody can read is not one.
fn exempt_list(
    root: &Path,
) -> Result<std::collections::BTreeSet<String>, String> {
    let path = "dev/tdd-order.exempt";
    let text = std::fs::read_to_string(root.join(path))
        .map_err(|_| format!("{path} does not exist; nothing was checked"))?;
    tdd_order::exempt(&text).map_err(|e| format!("{path} {e}"))
}

fn default_range(root: &Path) -> String {
    let upstream =
        git(root, &["rev-parse", "--verify", "--quiet", "@{upstream}"]);
    if upstream.is_some() {
        "@{upstream}..HEAD"
    } else {
        "HEAD"
    }
    .to_owned()
}

/// The range's commits, oldest first, each with the paths it adds
/// (renames detected, so a moved file adds no code).
fn commits(root: &Path, range: &str) -> Option<Vec<tdd_order::Commit>> {
    let ids = git(root, &["rev-list", "--reverse", range])?;
    ids.lines().map(|id| commit_at(root, id)).collect()
}

/// One commit's subject and the paths it adds.
fn commit_at(root: &Path, id: &str) -> Option<tdd_order::Commit> {
    let subject = git(root, &["log", "-1", "--format=%s", id])?;
    Some(tdd_order::Commit {
        id: id.to_owned(),
        subject: subject.trim().to_owned(),
        added: added_paths(root, id)?,
    })
}

/// `git show` for the paths a commit adds, renames detected (`-M`), so a
/// moved file adds no code.
const SHOW_ADDED: [&str; 5] = [
    "show",
    "-M",
    "--diff-filter=A",
    "--name-only",
    "--pretty=format:",
];

/// The paths a commit adds.
fn added_paths(root: &Path, id: &str) -> Option<Vec<String>> {
    let mut show = SHOW_ADDED.to_vec();
    show.push(id);
    let out = git(root, &show)?;
    Some(
        out.lines()
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect(),
    )
}
