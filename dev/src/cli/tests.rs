use super::run;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

type Res = Result<(), Box<dyn Error>>;

/// A scratch repository under the system temp dir, unique per test.
fn scratch(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let dir = std::env::temp_dir()
        .join(format!("chertmem-dev-{}-{name}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(dir.join(".private"))?;
    Ok(dir)
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn unknown_verb_is_usage() -> Res {
    let root = scratch("unknown")?;
    assert_eq!(run(&args(&["frobnicate"]), &root), 2);
    Ok(())
}

#[test]
fn name_guard_without_denylist_holds() -> Res {
    let root = scratch("nodeny")?;
    fs::remove_dir_all(root.join(".private"))?;
    fs::write(root.join("a.md"), "secretproj\n")?;
    assert_eq!(run(&args(&["name-guard", "a.md"]), &root), 0);
    Ok(())
}

#[test]
fn name_guard_planted_name_is_a_finding() -> Res {
    let root = scratch("planted")?;
    fs::write(root.join(".private/denylist"), "secretproj\n")?;
    fs::write(root.join("a.md"), "see SecretProj\n")?;
    assert_eq!(run(&args(&["name-guard", "a.md"]), &root), 1);
    Ok(())
}

#[test]
fn name_guard_clean_twin_holds() -> Res {
    let root = scratch("clean")?;
    fs::write(root.join(".private/denylist"), "secretproj\n")?;
    fs::write(root.join("a.md"), "see public docs\n")?;
    assert_eq!(run(&args(&["name-guard", "a.md"]), &root), 0);
    Ok(())
}

/// `git` in `dir`, with the hook's `GIT_*` variables dropped, so a test
/// run inside a hook never reaches the outer repository (`paradigm/V15`).
fn git(dir: &std::path::Path, list: &[&str]) -> Res {
    let mut cmd = std::process::Command::new("git");
    for (k, _) in std::env::vars_os() {
        if k.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(k);
        }
    }
    let ok = cmd.current_dir(dir).args(list).status()?.success();
    assert!(ok, "git {list:?} failed");
    Ok(())
}

#[test]
fn no_core_worktree_planted_is_a_finding() -> Res {
    let root = scratch("worktree-planted")?;
    git(&root, &["init", "-q"])?;
    git(&root, &["config", "core.worktree", "/tmp/elsewhere"])?;
    assert_eq!(run(&args(&["no-core-worktree"]), &root), 1);
    Ok(())
}

#[test]
fn no_core_worktree_clean_twin_holds() -> Res {
    let root = scratch("worktree-clean")?;
    git(&root, &["init", "-q"])?;
    assert_eq!(run(&args(&["no-core-worktree"]), &root), 0);
    Ok(())
}

#[test]
fn lock_graph_planted_second_nixpkgs_is_a_finding() -> Res {
    let root = scratch("lock-planted")?;
    let node = r#"{"original":{"repo":"nixpkgs"}}"#;
    let pin = r#"{"original":{"repo":"nixpkgs-lock"}}"#;
    let lock = format!(
        r#"{{"nodes":{{"nixpkgs":{node},"nixpkgs_2":{node},"nixpkgs-lock":{pin}}}}}"#
    );
    fs::write(root.join("flake.lock"), lock)?;
    assert_eq!(run(&args(&["lock-graph"]), &root), 1);
    Ok(())
}

#[test]
fn lock_graph_clean_twin_holds() -> Res {
    let root = scratch("lock-clean")?;
    let lock = r#"{"nodes":{"nixpkgs":{"original":{"repo":"nixpkgs"}},"nixpkgs-lock":{"original":{"repo":"nixpkgs-lock"}}}}"#;
    fs::write(root.join("flake.lock"), lock)?;
    assert_eq!(run(&args(&["lock-graph"]), &root), 0);
    Ok(())
}

#[test]
fn lock_graph_without_lock_is_unreadable_input() -> Res {
    let root = scratch("lock-missing")?;
    assert_eq!(run(&args(&["lock-graph"]), &root), 2);
    Ok(())
}

#[test]
fn rust_mirror_planted_module_without_mirror_is_a_finding() -> Res {
    let root = scratch("mirror-planted")?;
    git(&root, &["init", "-q"])?;
    fs::write(root.join("Cargo.toml"), "")?;
    fs::create_dir_all(root.join("src"))?;
    fs::write(root.join("src/a.rs"), "pub fn f() {}\n")?;
    git(&root, &["add", "-A"])?;
    assert_eq!(run(&args(&["rust-mirror"]), &root), 1);
    Ok(())
}

#[test]
fn rust_mirror_clean_twin_holds() -> Res {
    let root = scratch("mirror-clean")?;
    git(&root, &["init", "-q"])?;
    fs::write(root.join("Cargo.toml"), "")?;
    fs::create_dir_all(root.join("src/a"))?;
    fs::write(
        root.join("src/a.rs"),
        "#[cfg(test)]\nmod tests;\npub fn f() {}\n",
    )?;
    fs::write(root.join("src/a/tests.rs"), "")?;
    git(&root, &["add", "-A"])?;
    assert_eq!(run(&args(&["rust-mirror"]), &root), 0);
    Ok(())
}

/// Identity and no hooks, so a scratch commit needs no global config.
const COMMIT: [&str; 7] = [
    "-c",
    "user.name=t",
    "-c",
    "user.email=t@t",
    "-c",
    "core.hooksPath=/dev/null",
    "commit",
];

fn commit_all(dir: &std::path::Path, subject: &str) -> Res {
    git(dir, &["add", "-A"])?;
    let mut a = COMMIT.to_vec();
    a.extend(["-q", "-m", subject]);
    git(dir, &a)
}

/// A fresh repository with an empty `dev/tdd-order.exempt`.
fn exempt_repo(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let root = scratch(name)?;
    git(&root, &["init", "-q"])?;
    fs::create_dir_all(root.join("dev"))?;
    fs::write(root.join("dev/tdd-order.exempt"), "# none\n")?;
    Ok(root)
}

#[test]
fn tdd_order_planted_code_before_test_is_a_finding() -> Res {
    let root = exempt_repo("tdd-planted")?;
    fs::write(root.join("a.rs"), "pub fn f() {}\n")?;
    commit_all(&root, "feat: code first")?;
    assert_eq!(run(&args(&["tdd-order", "HEAD"]), &root), 1);
    Ok(())
}

#[test]
fn tdd_order_clean_twin_holds() -> Res {
    let root = exempt_repo("tdd-clean")?;
    fs::write(root.join("a_test.rs"), "")?;
    commit_all(&root, "test: RED first")?;
    fs::write(root.join("a.rs"), "pub fn f() {}\n")?;
    commit_all(&root, "feat: GREEN")?;
    assert_eq!(run(&args(&["tdd-order", "HEAD"]), &root), 0);
    Ok(())
}

#[test]
fn tdd_order_without_exempt_file_is_a_finding() -> Res {
    let root = scratch("tdd-noexempt")?;
    git(&root, &["init", "-q"])?;
    fs::write(root.join("a.md"), "x\n")?;
    commit_all(&root, "docs: x")?;
    assert_eq!(run(&args(&["tdd-order", "HEAD"]), &root), 1);
    Ok(())
}

/// A message with a body and the stage trailer under `subject`.
fn commit_message(subject: &str) -> String {
    format!(
        "{subject}\n\nWhy it changed.\n\nParadigm-Stage: wave-1\n\
         Co-Authored-By: A <a@example.com>\n"
    )
}

#[test]
fn commit_msg_planted_long_subject_is_a_finding() -> Res {
    let root = scratch("msg-planted")?;
    let subject = format!("feat: {}", "x".repeat(67));
    fs::write(root.join("MSG"), commit_message(&subject))?;
    assert_eq!(run(&args(&["commit-msg", "MSG"]), &root), 1);
    Ok(())
}

#[test]
fn commit_msg_clean_twin_holds() -> Res {
    let root = scratch("msg-clean")?;
    let subject = format!("feat: {}", "x".repeat(66));
    fs::write(root.join("MSG"), commit_message(&subject))?;
    assert_eq!(run(&args(&["commit-msg", "MSG"]), &root), 0);
    Ok(())
}

#[test]
fn commit_msg_unreadable_file_is_usage() -> Res {
    let root = scratch("msg-missing")?;
    assert_eq!(run(&args(&["commit-msg", "MSG"]), &root), 2);
    assert_eq!(run(&args(&["commit-msg"]), &root), 2);
    Ok(())
}
