//! `privacy.exclude`: path globs whose events are never captured (src/hook
//! V1, root V8).
//!
//! A path under the project root is matched relative to it. A glob
//! without `/` matches any 1 component (`.env`, `*.pem`); one with `/`
//! matches the path or any leading part of it, so a directory covers
//! what is under it. `*` and `?` stay within a component, `**` crosses
//! them, `**/` also matches no directory.

#[cfg(test)]
mod tests;

use std::path::Path;

/// Whether any of `paths` matches any of `globs`.
pub(super) fn excluded(
    globs: &[String],
    root: &Path,
    paths: &[String],
) -> bool {
    let relative: Vec<String> = paths.iter().map(|p| within(root, p)).collect();
    globs.iter().any(|g| relative.iter().any(|p| covers(g, p)))
}

/// `path` relative to `root` when under it, without a leading `./`.
fn within(root: &Path, path: &str) -> String {
    let path = path.strip_prefix("./").unwrap_or(path);
    let rel = Path::new(path).strip_prefix(root).ok();
    rel.map_or_else(|| path.to_owned(), |r| r.to_string_lossy().into_owned())
}

/// Whether `glob` covers `path`: a component, or a leading part.
fn covers(glob: &str, path: &str) -> bool {
    if !glob.contains('/') {
        return path.split('/').any(|part| matches(glob, part));
    }
    let ends = path.match_indices('/').map(|(at, _)| at);
    let mut leading = ends.filter_map(|at| path.get(..at));
    matches(glob, path) || leading.any(|part| matches(glob, part))
}

/// Whether `glob` matches all of `path`.
fn matches(glob: &str, path: &str) -> bool {
    wild(glob.as_bytes(), path.as_bytes())
}

fn wild(glob: &[u8], text: &[u8]) -> bool {
    match glob.split_first() {
        None => text.is_empty(),
        Some((b'*', rest)) => star(rest, text),
        Some((&want, rest)) => {
            text.split_first().is_some_and(|(&got, tail)| {
                let one = want == got || (want == b'?' && got != b'/');
                one && wild(rest, tail)
            })
        }
    }
}

/// After a `*`: `rest` matches at some later point of `text`.
fn star(rest: &[u8], text: &[u8]) -> bool {
    let (deep, rest) =
        rest.strip_prefix(b"*").map_or((false, rest), |r| (true, r));
    let no_dir = rest.strip_prefix(b"/").filter(|_| deep);
    if no_dir.is_some_and(|after| wild(after, text)) {
        return true;
    }
    (0..=text.len()).any(|at| {
        let skipped = text.get(..at).unwrap_or_default();
        let tail = text.get(at..).unwrap_or_default();
        (deep || !skipped.contains(&b'/')) && wild(rest, tail)
    })
}
