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
