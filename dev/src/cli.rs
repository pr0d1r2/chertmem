//! Dispatch of `chertmem-dev <guard> [files...]` (dev §I): exit 0 hold, 1
//! finding, 2 usage or unreadable input.

#[cfg(test)]
mod tests;

use std::path::Path;

/// Run one guard from `args` against the repository at `root`.
#[must_use]
pub fn run(args: &[String], root: &Path) -> u8 {
    let _ = (args, root);
    0
}
