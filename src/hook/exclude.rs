//! `privacy.exclude`: path globs whose events are never captured (src/hook
//! V1, root V8).

#[cfg(test)]
mod tests;

use std::path::Path;

/// Whether any of `paths` matches any of `globs`.
pub(super) fn excluded(
    globs: &[String],
    root: &Path,
    paths: &[String],
) -> bool {
    let _ = root;
    globs.iter().any(|g| paths.iter().any(|p| matches(g, p)))
}

/// Whether `glob` matches all of `path`.
fn matches(glob: &str, path: &str) -> bool {
    glob == path
}
