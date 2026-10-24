//! `no-core-worktree`: the shared `.git/config` sets no `core.worktree`,
//! which would redirect every worktree (dev V4, `paradigm/V15`).

#[cfg(test)]
mod tests;

/// The `core.worktree` value set in a git config's text, if any. Section
/// and key names match case-insensitively, as git reads them.
#[must_use]
pub fn core_worktree(config: &str) -> Option<String> {
    let mut in_core = false;
    for raw in config.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_core = line.eq_ignore_ascii_case("[core]");
        } else if in_core && let Some(v) = value(line, "worktree") {
            return Some(v);
        }
    }
    None
}

/// The value of `key = value`, when the line sets `key`.
fn value(line: &str, key: &str) -> Option<String> {
    let (k, v) = line.split_once('=')?;
    k.trim()
        .eq_ignore_ascii_case(key)
        .then(|| v.trim().to_owned())
}
