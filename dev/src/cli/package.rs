//! `package-list` (dev T12): `cargo package --list` against the rules
//! of `package::findings`; a failed cargo call is reported with its
//! stderr (rekall#47).

#[cfg(test)]
mod tests;

use std::path::Path;

use crate::{external, package};

/// The tarball's file list, without building it.
const LIST: [&str; 7] = [
    "package",
    "-p",
    "chertmem",
    "--list",
    "--allow-dirty",
    "--locked",
    "--offline",
];

/// The package verb `verb` with `rest`; `None` when it is not one.
pub(super) fn run(verb: &str, rest: &[String], root: &Path) -> Option<u8> {
    (verb == "package-list" && rest.is_empty())
        .then(|| package_list(root, "cargo"))
}

/// `package-list` with `cargo` as the cargo binary: exit 1 on a
/// finding, 2 when cargo or the README fails.
pub(super) fn package_list(root: &Path, cargo: &str) -> u8 {
    match findings(root, cargo) {
        Ok(found) => {
            for f in &found {
                eprintln!("package-list: {f}");
            }
            u8::from(!found.is_empty())
        }
        Err(why) => {
            eprintln!("package-list: {why}");
            2
        }
    }
}

fn findings(root: &Path, cargo: &str) -> Result<Vec<String>, String> {
    let list = external::stdout(root, cargo, &LIST)?;
    let readme = std::fs::read_to_string(root.join("README.md"))
        .map_err(|e| format!("cannot read README.md: {e}"))?;
    Ok(package::findings(&list, &readme))
}
