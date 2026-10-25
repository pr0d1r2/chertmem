//! The bench verbs: `bench-record` (benches T4). Paths are the
//! repository's own; criterion's output is read from `target/criterion`.

#[cfg(test)]
mod tests;

use std::path::Path;

pub(super) const BASELINE: &str = "benches/baseline.toml";
/// The suite index the bench run writes beside its group dirs.
pub(super) const INDEX: &str = "chert-suites.tsv";

/// The bench verb `verb` with `rest`; `None` when it is not one.
pub(super) fn run(verb: &str, rest: &[String], root: &Path) -> Option<u8> {
    match (verb, rest) {
        ("bench-record", []) => Some(recorded(root)),
        _ => None,
    }
}

/// `bench-record`: criterion's last run into `benches/baseline.toml`.
fn recorded(root: &Path) -> u8 {
    let _ = (root, BASELINE, INDEX);
    2
}
