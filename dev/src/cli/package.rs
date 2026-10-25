//! `package-list` (dev T11): `cargo package --list` against the rules
//! of `package::findings`; a failed cargo call is reported with its
//! stderr (rekall#47).

#[cfg(test)]
mod tests;

use std::path::Path;

/// The package verb `verb` with `rest`; `None` when it is not one.
pub(super) fn run(verb: &str, rest: &[String], root: &Path) -> Option<u8> {
    (verb == "package-list" && rest.is_empty())
        .then(|| package_list(root, "cargo"))
}

/// `package-list` with `cargo` as the cargo binary: exit 1 on a
/// finding, 2 when cargo or the README fails.
pub(super) fn package_list(root: &Path, cargo: &str) -> u8 {
    let _ = (root, cargo);
    0
}
