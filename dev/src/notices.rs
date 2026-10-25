//! `notices` (dev T11): the Rust crates part of
//! `THIRD-PARTY-NOTICES.md`, rendered from `cargo tree` and `cargo
//! metadata` and the license files each crate's source ships; the same
//! bytes for the same inputs.

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::PathBuf;

use tinyjson::JsonValue;

/// The systems `chert` is released for (`paradigm/V28`): the crate set
/// is theirs, the same on every host.
pub const TARGETS: [&str; 4] = [
    "aarch64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
];

const BEGIN: &str = "<!-- BEGIN crates -->";
const END: &str = "<!-- END crates -->";

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
    tree.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let name = words.next()?;
            let version = words.next()?.strip_prefix('v')?;
            Some((name.to_owned(), version.to_owned()))
        })
        .collect()
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
    let root: JsonValue = json.parse().map_err(|e| format!("{e}"))?;
    let root: &HashMap<String, JsonValue> =
        root.get().ok_or("cargo metadata is not an object")?;
    let packages: &Vec<JsonValue> = root
        .get("packages")
        .and_then(JsonValue::get)
        .ok_or("cargo metadata has no packages")?;
    packages.iter().map(package).collect()
}

fn package(p: &JsonValue) -> Result<((String, String), Meta), String> {
    let map: &HashMap<String, JsonValue> =
        p.get().ok_or("a package is not an object")?;
    let field = |k: &str| map.get(k).and_then(JsonValue::get::<String>);
    let need = |k: &str| field(k).ok_or(format!("a package has no {k}"));
    let manifest = PathBuf::from(need("manifest_path")?);
    let meta = Meta {
        license: field("license").cloned(),
        repository: field("repository").cloned(),
        dir: manifest.parent().map(PathBuf::from).unwrap_or_default(),
        registry: field("source").is_some(),
    };
    Ok(((need("name")?.clone(), need("version")?.clone()), meta))
}

/// The stem's suffix of a license or notice file name: `Some("")` for
/// one that names no license (`LICENSE`, `COPYING`, `NOTICE.txt`),
/// `Some("MIT")` for `LICENSE-MIT`, `None` for any other file.
fn suffix(name: &str) -> Option<String> {
    let up = name.to_ascii_uppercase();
    let stems = ["LICENSE", "LICENCE", "COPYING", "NOTICE"];
    let rest = stems.iter().find_map(|s| up.strip_prefix(s))?;
    let rest = [".TXT", ".MD"]
        .iter()
        .find_map(|e| rest.strip_suffix(e))
        .unwrap_or(rest);
    Some(rest.trim_start_matches(['-', '.', '_']).to_owned())
}

/// Whether file `name` belongs to the `chosen` license: it names none,
/// or it names one of the chosen ids.
fn fits(name: &str, chosen: &str) -> bool {
    let Some(s) = suffix(name) else {
        return false;
    };
    s.is_empty()
        || chosen.split(" AND ").any(|id| {
            let id = id.split(' ').next().unwrap_or(id).to_ascii_uppercase();
            id == s || id.starts_with(&format!("{s}-"))
        })
}

/// Of a crate's top-level `files`, the license and notice files that
/// go on the page: those of the `chosen` license, or every one when
/// none fits it. Sorted.
#[must_use]
pub fn wanted(files: &[String], chosen: &str) -> Vec<String> {
    let mut all: Vec<String> = files
        .iter()
        .filter(|f| suffix(f).is_some())
        .cloned()
        .collect();
    all.sort();
    let fit: Vec<String> =
        all.iter().filter(|f| fits(f, chosen)).cloned().collect();
    if fit.is_empty() { all } else { fit }
}

/// `text` as the page keeps it: LF endings, tabs as 4 spaces, no
/// trailing spaces, no blank lines at either end.
#[must_use]
pub fn clean(text: &str) -> String {
    let text = text.replace("\r\n", "\n").replace('\t', "    ");
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    let body = lines.join("\n");
    body.trim_matches('\n').to_owned()
}

/// The generated block, markers included.
#[must_use]
pub fn render(entries: &[Entry]) -> String {
    let ids = text_ids(entries);
    let mut page = head(entries);
    for e in entries {
        row(&mut page, e, &ids);
    }
    texts(&mut page, entries, &ids);
    page.push('\n');
    page.push_str(END);
    page.push('\n');
    page
}

/// Each distinct text's number, from 1, in order of first use.
fn text_ids(entries: &[Entry]) -> BTreeMap<&str, usize> {
    let mut ids = BTreeMap::new();
    for t in entries.iter().flat_map(|e| &e.texts) {
        let next = ids.len().saturating_add(1);
        ids.entry(t.as_str()).or_insert(next);
    }
    ids
}

/// The block's head; `{default}`, `{bpe}` and `{systems}` are filled in.
const HEAD: &str = "\
<!-- BEGIN crates -->

## Rust crates

<!-- Generated by `cargo run -q -p chertmem-dev -- notices`; do not edit. -->

This section is generated from `Cargo.lock` by `cargo run -q -p
chertmem-dev -- notices`; the commit gate's `notices --check` fails
while it is stale. It lists every crate cargo builds into `chert`,
build scripts and proc macros included, on the release systems:

{systems}

That is {default} crates in the default build, and {bpe} more with
feature `bpe` (marked `bpe`).

Where a crate offers a choice of licenses, chert takes MIT whenever it
is offered, otherwise the most permissive option, and keeps every term
joined by `AND`. The license and notice files each crate's source ships
for that choice follow the table, each distinct text once.

| Crate | Version | License | Feature | Repository | Texts |
| --- | --- | --- | --- | --- | --- |
";

fn head(entries: &[Entry]) -> String {
    let bpe = entries.iter().filter(|e| e.bpe_only).count();
    let default = entries.len().saturating_sub(bpe);
    let systems = TARGETS.map(|t| format!("- `{t}`")).join("\n");
    HEAD.replace("{systems}", &systems)
        .replace("{default}", &default.to_string())
        .replace("{bpe}", &bpe.to_string())
}

fn row(page: &mut String, e: &Entry, ids: &BTreeMap<&str, usize>) {
    let feature = if e.bpe_only { "`bpe`" } else { "default" };
    let repo = e
        .repository
        .as_ref()
        .map_or("-".to_owned(), |r| format!("<{r}>"));
    let refs = refs(e, ids);
    let (name, version, license) = (&e.name, &e.version, &e.license);
    let _ = writeln!(
        page,
        "| {name} | {version} | {license} | {feature} | {repo} | {refs} |"
    );
}

/// Links to the entry's texts, or `none shipped`.
fn refs(e: &Entry, ids: &BTreeMap<&str, usize>) -> String {
    let refs: Vec<String> = e
        .texts
        .iter()
        .filter_map(|t| ids.get(t.as_str()))
        .map(|n| format!("[{n}](#text-{n})"))
        .collect();
    if refs.is_empty() {
        return "none shipped".to_owned();
    }
    refs.join(", ")
}

fn texts(page: &mut String, entries: &[Entry], ids: &BTreeMap<&str, usize>) {
    let mut by_id: Vec<(&usize, &&str)> =
        ids.iter().map(|(t, n)| (n, t)).collect();
    by_id.sort();
    for (n, text) in by_id {
        let users: Vec<String> = entries
            .iter()
            .filter(|e| e.texts.iter().any(|t| t == *text))
            .map(|e| format!("`{} {}`", e.name, e.version))
            .collect();
        let users = users.join(", ");
        let fence = fence(text);
        let _ = write!(page, "\n### Text {n}\n\nShipped by {users}.\n\n");
        let _ = write!(page, "{fence}text\n{text}\n{fence}\n");
    }
}

/// A backtick fence longer than any backtick run in `text`.
fn fence(text: &str) -> String {
    let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    "`".repeat(longest.saturating_add(1).max(3))
}

/// `doc` with its generated block replaced by `block`.
///
/// # Errors
/// `doc` has no `BEGIN crates` marker followed by an `END crates` one.
pub fn splice(doc: &str, block: &str) -> Result<String, String> {
    let missing = || format!("no {BEGIN} ... {END} block");
    let (before, rest) = doc.split_once(BEGIN).ok_or_else(missing)?;
    let (_, after) = rest.split_once(END).ok_or_else(missing)?;
    let after = after.strip_prefix('\n').unwrap_or(after);
    Ok(format!("{before}{block}{after}"))
}
