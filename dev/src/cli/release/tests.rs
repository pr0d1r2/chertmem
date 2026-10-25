use super::{NOTICES, notices, page};
use crate::cli::run;
use std::error::Error;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

type Res = Result<(), Box<dyn Error>>;

fn scratch(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let dir = std::env::temp_dir().join(format!(
        "chertmem-dev-{}-release-{name}",
        std::process::id()
    ));
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

const DOC: &str = "# Third-party notices\n\n## SQLite\n\nhand\n\n\
                   <!-- BEGIN crates -->\n<!-- END crates -->\n";

/// A stand-in for cargo: `tree` and `metadata` print the files beside
/// it, the default tree unless `bpe` is asked for; anything else fails
/// as cargo does, with stderr and exit 101.
const FAKE: &str = r#"#!/bin/sh
dir=$(dirname "$0")
case "$1" in
  metadata) cat "$dir/meta.json" ;;
  tree) case "$*" in *bpe*) cat "$dir/tree-bpe.txt" ;; *) cat "$dir/tree.txt" ;; esac ;;
  *) echo "error: no such command: $1" >&2; exit 101 ;;
esac
"#;

/// A repository with crate `a` (MIT OR Apache-2.0, 2 license files)
/// and `t`, only in `bpe`, with no license file.
fn repo(name: &str) -> Result<(PathBuf, String), Box<dyn Error>> {
    let root = scratch(name)?;
    let reg = root.join("reg");
    fs::create_dir_all(reg.join("a-1.0.0"))?;
    fs::create_dir_all(reg.join("t-2.0.0/src"))?;
    fs::write(
        reg.join("a-1.0.0/LICENSE-MIT"),
        "MIT License\r\n\r\n(c) A\r\n",
    )?;
    fs::write(reg.join("a-1.0.0/LICENSE-APACHE"), "Apache\n")?;
    fs::write(root.join("meta.json"), meta(&reg))?;
    let cargo = fake(&root)?;
    Ok((root, cargo))
}

/// The fake cargo, its 2 trees and the page with an empty block.
fn fake(root: &Path) -> Result<String, Box<dyn Error>> {
    let own = "own v0.0.0 (/x)\na v1.0.0\n";
    fs::write(root.join("tree.txt"), own)?;
    fs::write(root.join("tree-bpe.txt"), format!("{own}t v2.0.0\n"))?;
    fs::write(root.join(NOTICES), DOC)?;
    let cargo = root.join("cargo");
    fs::write(&cargo, FAKE)?;
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755))?;
    Ok(cargo.display().to_string())
}

fn meta(reg: &Path) -> String {
    let pkg = |n: &str, v: &str, lic: &str, src: &str| {
        let dir = reg.join(format!("{n}-{v}"));
        format!(
            r#"{{"name":"{n}","version":"{v}","license":{lic},"repository":"https://x.test/{n}","manifest_path":"{}/Cargo.toml","source":{src}}}"#,
            dir.display()
        )
    };
    let reg_src = r#""registry+https://github.com/rust-lang/crates.io-index""#;
    format!(
        r#"{{"packages":[{},{},{}]}}"#,
        pkg("a", "1.0.0", r#""MIT OR Apache-2.0""#, reg_src),
        pkg("t", "2.0.0", "null", reg_src),
        pkg("own", "0.0.0", "null", "null"),
    )
}

#[test]
fn notices_writes_the_block_and_the_check_then_holds() -> Res {
    let (root, cargo) = repo("write")?;
    assert_eq!(notices(&root, &cargo, true), 1, "the empty block is stale");
    assert_eq!(notices(&root, &cargo, false), 0);
    written(&fs::read_to_string(root.join(NOTICES))?);
    assert_eq!(notices(&root, &cargo, true), 0, "clean twin holds");
    Ok(())
}

/// The page `repo` makes: the hand part kept, `a` with its MIT text
/// only, `t` with none, chert's own package left out.
fn written(page: &str) {
    assert!(page.starts_with("# Third-party notices\n\n## SQLite\n\nhand\n"));
    let row =
        "| a | 1.0.0 | MIT | default | <https://x.test/a> | [1](#text-1) |";
    let t = "| t | 2.0.0 | see its texts | `bpe` | <https://x.test/t> | none";
    let text = "```text\nMIT License\n\n(c) A\n```";
    for want in [row, t, text] {
        assert!(page.contains(want), "{want}\n---\n{page}");
    }
    assert!(!page.contains("Apache\n") && !page.contains("| own |"));
}

/// dev V6: the planted violation is a hand edit of a crate's version.
#[test]
fn notices_check_rejects_a_hand_edit() -> Res {
    let (root, cargo) = repo("edit")?;
    assert_eq!(notices(&root, &cargo, false), 0);
    let page = fs::read_to_string(root.join(NOTICES))?;
    fs::write(root.join(NOTICES), page.replace("| 1.0.0 |", "| 1.0.1 |"))?;
    assert_eq!(notices(&root, &cargo, true), 1);
    fs::set_permissions(root.join(NOTICES), fs::Permissions::from_mode(0o444))?;
    assert_eq!(notices(&root, &cargo, false), 2, "an unwritable page");
    Ok(())
}

/// rekall#47: a failing cargo call is reported with its stderr, never
/// read as an empty crate list.
#[test]
fn a_failing_cargo_call_is_loud() -> Res {
    let (root, cargo) = repo("fail")?;
    fs::write(
        root.join("cargo"),
        "#!/bin/sh\necho 'error: offline' >&2\nexit 101\n",
    )?;
    let got = page(&root, &cargo);
    assert!(got.is_err_and(|e| e.contains("(exit 101): error: offline")));
    assert_eq!(notices(&root, &cargo, true), 2);
    Ok(())
}

#[test]
fn a_crate_missing_from_metadata_or_a_page_without_block_is_an_error() -> Res {
    let (root, cargo) = repo("missing")?;
    fs::write(root.join("tree.txt"), "z v9.0.0\n")?;
    let got = page(&root, &cargo);
    assert!(got.is_err_and(|e| e.contains("z 9.0.0 is in cargo tree")));
    let (root, cargo) = repo("noblock")?;
    fs::write(root.join(NOTICES), "# Third-party notices\n")?;
    assert!(page(&root, &cargo).is_err_and(|e| e.contains("BEGIN crates")));
    fs::remove_file(root.join(NOTICES))?;
    assert_eq!(notices(&root, &cargo, false), 2);
    Ok(())
}

#[test]
fn a_license_dir_that_cannot_be_listed_is_an_error() -> Res {
    let (root, cargo) = repo("nodir")?;
    fs::remove_dir_all(root.join("reg/a-1.0.0"))?;
    assert!(page(&root, &cargo).is_err_and(|e| e.contains("cannot list")));
    Ok(())
}

#[test]
fn notices_verb_runs_cargo_and_rejects_other_flags() -> Res {
    let root = scratch("verb")?;
    let args =
        |l: &[&str]| l.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    assert_eq!(run(&args(&["notices", "--frobnicate"]), &root), 2);
    assert_eq!(
        run(&args(&["notices"]), &root),
        2,
        "no page, no cargo project"
    );
    assert_eq!(run(&args(&["notices", "--check"]), &root), 2);
    Ok(())
}
