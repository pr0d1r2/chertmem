//! `rust-mirror`: every Rust module with logic has its unit tests beside
//! it, wired in, and every `tests.rs` has its module (dev V7, a port of
//! the xenolith v0.1.0 rule lablith copies as shell).
//!
//! The test is textual, as the original's: `//` comments (and so doc
//! comments) are dropped, block comments and string literals are not
//! parsed, so a `fn` in either reads as code.

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

/// The text on one line, `//` comments dropped.
fn flat(text: &str) -> String {
    text.lines()
        .map(|l| l.split_once("//").map_or(l, |(code, _)| code))
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Names of every `fn` defined with a body, `//` comments dropped.
#[must_use]
pub fn bodied_fns(text: &str) -> Vec<String> {
    let flat = flat(text);
    let mut out = Vec::new();
    let mut rest = flat.as_str();
    while let Some((name, after)) = next_fn(rest) {
        let body = after.find(['{', ';']).and_then(|i| after.get(i..));
        if body.is_some_and(|b| b.starts_with('{')) {
            out.push(name.to_owned());
        }
        rest = after;
    }
    out
}

/// The next `fn <name>` in `s`, as the name and the text after it.
fn next_fn(s: &str) -> Option<(&str, &str)> {
    s.match_indices("fn").find_map(|(i, _)| {
        let before = s.get(..i)?.chars().next_back();
        if before.is_some_and(is_ident) {
            return None;
        }
        let tail = s.get(i..)?.strip_prefix("fn")?;
        let name_start = tail.strip_prefix(char::is_whitespace)?.trim_start();
        let len = name_start
            .find(|c| !is_ident(c))
            .unwrap_or(name_start.len());
        let name = name_start.get(..len).filter(|n| !n.is_empty())?;
        Some((name, name_start.get(len..)?))
    })
}

/// Whether the module declares `#[cfg(test)] mod tests;`.
#[must_use]
pub fn wired(text: &str) -> bool {
    let flat = flat(text);
    flat.match_indices("#[cfg(test)]").any(|(i, m)| {
        flat.get(i..)
            .and_then(|s| s.strip_prefix(m))
            .map(str::trim_start)
            .and_then(|s| s.strip_prefix("mod"))
            .and_then(|s| s.strip_prefix(char::is_whitespace))
            .map(str::trim_start)
            .and_then(|s| s.strip_prefix("tests"))
            .is_some_and(|s| s.trim_start().starts_with(';'))
    })
}

fn exempt(path: &str, crates: &[String]) -> bool {
    path == "build.rs"
        || path.ends_with("/build.rs")
        || path.starts_with("vendor/")
        || path.contains("/vendor/")
        || crates
            .iter()
            .any(|c| path.starts_with(&format!("{c}tests/")))
}

fn is_mirror(path: &str) -> bool {
    path == "tests.rs" || path.ends_with("/tests.rs")
}

/// The directory part of a path, with its trailing `/` (`""` at root).
fn dir_of(path: &str) -> &str {
    path.rfind('/')
        .and_then(|i| path.get(..=i))
        .unwrap_or_default()
}

/// Every broken mirror among the tracked `.rs` files (path to text),
/// given the crate roots (`""` for the repository root, `"dev/"` ...).
#[must_use]
pub fn findings(
    rs: &BTreeMap<String, String>,
    crates: &[String],
) -> Vec<String> {
    let live = rs.iter().filter(|(p, _)| !exempt(p, crates));
    let mut out: Vec<String> = live
        .clone()
        .filter(|(p, _)| !is_mirror(p))
        .filter_map(|(p, t)| module_finding(rs, p, t))
        .collect();
    out.extend(
        live.filter(|(p, _)| is_mirror(p))
            .filter_map(|(p, _)| mirror_finding(rs, p)),
    );
    out
}

/// Direction one: a module with logic has its wired mirror.
fn module_finding(
    rs: &BTreeMap<String, String>,
    path: &str,
    text: &str,
) -> Option<String> {
    let fns = bodied_fns(text);
    if fns.is_empty() {
        return None;
    }
    let name = path.rsplit('/').next().unwrap_or(path);
    if name == "main.rs" {
        return (fns != ["main"]).then(|| not_a_shim(path, &fns));
    }
    mirror_state(rs, path, &expected_mirror(path, name), text)
}

/// A module with logic: its mirror missing, or present but unwired.
fn mirror_state(
    rs: &BTreeMap<String, String>,
    path: &str,
    expected: &str,
    text: &str,
) -> Option<String> {
    if !rs.contains_key(expected) {
        return Some(format!(
            "{path} has logic and no unit tests. Expected them at {expected}, wired by #[cfg(test)] mod tests; (dev V7)."
        ));
    }
    (!wired(text)).then(|| format!(
        "{expected} is never compiled: {path} does not declare #[cfg(test)] mod tests; so its tests pass by not existing (dev V7)."
    ))
}

fn expected_mirror(path: &str, name: &str) -> String {
    if name == "lib.rs" || name == "mod.rs" {
        return format!("{}tests.rs", dir_of(path));
    }
    format!("{}/tests.rs", path.strip_suffix(".rs").unwrap_or(path))
}

fn not_a_shim(path: &str, fns: &[String]) -> String {
    format!(
        "{path} is not a shim: it defines fn {}, where only fn main belongs. main.rs has no mirror of its own, so move the logic into the library and leave fn main calling it (dev V7).",
        fns.join(" ")
    )
}

/// Direction two: a mirror has the module that wires it. A module with
/// logic and no wiring was reported by direction one, so only a module
/// without logic is named here: each defect once.
fn mirror_finding(rs: &BTreeMap<String, String>, path: &str) -> Option<String> {
    let dir = dir_of(path);
    let Some((owner, text)) = owner_of(rs, dir) else {
        return Some(orphan(path, dir));
    };
    let silent = !wired(text) && bodied_fns(text).is_empty();
    silent.then(|| format!(
        "{path} is never compiled: {owner} does not declare #[cfg(test)] mod tests; (dev V7)."
    ))
}

fn owner_of<'a>(
    rs: &'a BTreeMap<String, String>,
    dir: &str,
) -> Option<(String, &'a String)> {
    let mut candidates = vec![format!("{dir}lib.rs"), format!("{dir}mod.rs")];
    if let Some(d) = dir.strip_suffix('/') {
        candidates.push(format!("{d}.rs"));
    }
    candidates
        .into_iter()
        .find_map(|c| rs.get(&c).map(|t| (c, t)))
}

fn orphan(path: &str, dir: &str) -> String {
    match dir.strip_suffix('/') {
        Some(d) => format!(
            "{path} tests nothing: neither {d}.rs nor {dir}mod.rs nor {dir}lib.rs exists. An orphan test passes forever: delete it, or restore the module it covered (dev V7)."
        ),
        None => format!(
            "{path} tests nothing: there is no lib.rs beside it (dev V7)."
        ),
    }
}
