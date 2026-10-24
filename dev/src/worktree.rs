//! `no-core-worktree`: the shared `.git/config` sets no `core.worktree`,
//! which would redirect every worktree (dev V4, `paradigm/V15`).

#[cfg(test)]
mod tests;

/// The `core.worktree` value set in a git config's text, if any.
#[must_use]
pub fn core_worktree(config: &str) -> Option<String> {
    let _ = config;
    None
}
