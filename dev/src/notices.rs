//! `notices` (dev T10): the Rust crates part of
//! `THIRD-PARTY-NOTICES.md`, rendered from `cargo tree` and `cargo
//! metadata` and the license files each crate's source ships; the same
//! bytes for the same inputs.

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// The systems `chert` is released for (`paradigm/V28`): the crate set
/// is theirs, the same on every host.
pub const TARGETS: [&str; 4] = [
    "aarch64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
];

/// A crate's row of `cargo metadata`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Meta {
    /// The `license` expression, if the manifest has one.
    pub license: Option<String>,
    /// The `repository` URL, if the manifest has one.
    pub repository: Option<String>,
    /// The directory of its `Cargo.toml`: the crate's source.
    pub dir: PathBuf,
    /// False for a path (workspace) package: chert's own.
    pub registry: bool,
}

/// A crate as the page lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Crate name.
    pub name: String,
    /// Crate version.
    pub version: String,
    /// The license chosen (`spdx::choose`), or why there is none.
    pub license: String,
    /// The repository URL, if any.
    pub repository: Option<String>,
    /// Shipped only with feature `bpe`.
    pub bpe_only: bool,
    /// Its license and notice texts, each `clean`.
    pub texts: Vec<String>,
}

/// (name, version) of every package in `cargo tree --prefix none
/// --format {p}` output.
#[must_use]
pub fn shipped(tree: &str) -> BTreeSet<(String, String)> {
    let _ = tree;
    BTreeSet::new()
}

/// Every package of `cargo metadata --format-version 1`, by (name,
/// version).
///
/// # Errors
/// The JSON does not parse, or a package lacks its name, version or
/// manifest path.
pub fn metadata(
    json: &str,
) -> Result<BTreeMap<(String, String), Meta>, String> {
    let _ = json;
    Ok(BTreeMap::new())
}

/// Of a crate's top-level `files`, the license and notice files that
/// go on the page. Sorted.
#[must_use]
pub fn wanted(files: &[String], chosen: &str) -> Vec<String> {
    let _ = (files, chosen);
    Vec::new()
}

/// `text` as the page keeps it.
#[must_use]
pub fn clean(text: &str) -> String {
    text.to_owned()
}

/// The generated block, markers included.
#[must_use]
pub fn render(entries: &[Entry]) -> String {
    let _ = entries;
    String::new()
}

/// `doc` with its generated block replaced by `block`.
///
/// # Errors
/// `doc` has no `BEGIN crates` marker followed by an `END crates` one.
pub fn splice(doc: &str, block: &str) -> Result<String, String> {
    let _ = block;
    Ok(doc.to_owned())
}
