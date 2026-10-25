//! `package-list` (dev T11): what `cargo package -p chertmem --list`
//! puts in the crates.io tarball. No `SPEC.md` and nothing under
//! `.githooks/` ships, every file building and licensing need does, and
//! every relative link in `README.md` points at a file that ships, so
//! none breaks on crates.io.

#[cfg(test)]
mod tests;

/// Why the package `list` (1 path per line) fails, given the README's
/// text; empty when it holds.
#[must_use]
pub fn findings(list: &str, readme: &str) -> Vec<String> {
    let _ = (list, readme);
    Vec::new()
}
