use super::package_list;
use crate::cli::run;
use std::error::Error;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

type Res = Result<(), Box<dyn Error>>;

/// A fake cargo printing `list.txt` for `package`, and failing as cargo
/// does for anything else.
const FAKE: &str = r#"#!/bin/sh
case "$1" in
  package) cat "$(dirname "$0")/list.txt" ;;
  *) echo "error: no such command: $1" >&2; exit 101 ;;
esac
"#;

const LIST: &str = "Cargo.lock\nCargo.toml\nLICENSE\nREADME.md\n\
                    THIRD-PARTY-NOTICES.md\nsrc/main.rs\n";

/// A repository whose package lists `list` and whose README is `readme`.
fn repo(
    name: &str,
    list: &str,
    readme: &str,
) -> Result<(PathBuf, String), Box<dyn Error>> {
    let root = std::env::temp_dir().join(format!(
        "chertmem-dev-{}-package-{name}",
        std::process::id()
    ));
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    fs::create_dir_all(&root)?;
    fs::write(root.join("list.txt"), list)?;
    fs::write(root.join("README.md"), readme)?;
    let cargo = root.join("cargo");
    fs::write(&cargo, FAKE)?;
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755))?;
    Ok((root, cargo.display().to_string()))
}

#[test]
fn a_clean_package_holds() -> Res {
    let (root, cargo) = repo("clean", LIST, "[MIT](LICENSE)\n")?;
    assert_eq!(package_list(&root, &cargo), 0);
    Ok(())
}

/// dev V6: the planted violation is a SPEC.md in the tarball.
#[test]
fn a_shipped_spec_is_a_finding() -> Res {
    let list = format!("{LIST}src/SPEC.md\n");
    let (root, cargo) = repo("spec", &list, "")?;
    assert_eq!(package_list(&root, &cargo), 1);
    Ok(())
}

#[test]
fn a_failing_cargo_or_a_missing_readme_is_unreadable_input() -> Res {
    let (root, cargo) = repo("fail", LIST, "")?;
    fs::write(root.join("cargo"), "#!/bin/sh\necho no >&2\nexit 101\n")?;
    assert_eq!(package_list(&root, &cargo), 2);
    let (root, cargo) = repo("noreadme", LIST, "")?;
    fs::remove_file(root.join("README.md"))?;
    assert_eq!(package_list(&root, &cargo), 2);
    Ok(())
}

#[test]
fn package_list_verb_takes_no_arguments() -> Res {
    let (root, _) = repo("verb", LIST, "")?;
    let args =
        |l: &[&str]| l.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    assert_eq!(run(&args(&["package-list", "x"]), &root), 2);
    assert_eq!(run(&args(&["package-list"]), &root), 2, "no cargo project");
    Ok(())
}
