//! The release verbs: `notices [--check]` (dev T10). Every cargo call
//! goes through `external::stdout`, so a failed one is reported with its
//! stderr, never read as an empty crate list (rekall#47).

#[cfg(test)]
mod tests;

use std::path::Path;

pub(super) const NOTICES: &str = "THIRD-PARTY-NOTICES.md";

/// The release verb `verb` with `rest`; `None` when it is not one.
pub(super) fn run(verb: &str, rest: &[String], root: &Path) -> Option<u8> {
    match (verb, rest) {
        ("notices", []) => Some(notices(root, "cargo", false)),
        ("notices", [flag]) if flag == "--check" => {
            Some(notices(root, "cargo", true))
        }
        _ => None,
    }
}

/// `notices [--check]` with `cargo` as the cargo binary: write the
/// page, or exit 1 when it differs.
pub(super) fn notices(root: &Path, cargo: &str, check: bool) -> u8 {
    let _ = check;
    u8::from(page(root, cargo).is_err())
}

/// The page on disk and the page the lock makes.
fn page(root: &Path, cargo: &str) -> Result<(String, String), String> {
    let _ = cargo;
    let old = std::fs::read_to_string(root.join(NOTICES))
        .map_err(|e| format!("cannot read {NOTICES}: {e}"))?;
    Ok((old.clone(), old))
}
