//! IDE wiring (src/install G): chert's hooks and MCP server registered
//! in, and removed from, an IDE's config. v1 IDE = Claude Code (D24).
//!
//! Every entry takes the config file as [`Target::settings`], so tests
//! edit temp files, never a real `~/.claude/settings.json`.

#[cfg(test)]
mod tests;

mod error;

use std::path::{Path, PathBuf};

pub use error::Error;

use crate::hook::Ide;

/// What [`install`] and [`uninstall`] edit.
#[derive(Debug, Clone, Copy)]
pub struct Target<'a> {
    /// The IDE config file, e.g. [`settings_path`].
    pub settings: &'a Path,
    /// The `chert` program the hooks and the MCP server run: a name on
    /// `PATH` or a path to the binary.
    pub chert: &'a str,
}

/// What a call did to the config file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Nothing to do: the file is as it was, byte for byte (V2).
    Unchanged,
    /// The file was written as JSON.
    Wrote,
    /// The original bytes are back, or a file chert created is gone
    /// (V1).
    Restored,
}

/// The config file `ide` reads under home dir `home`.
#[must_use]
pub fn settings_path(home: &Path, ide: Ide) -> PathBuf {
    match ide {
        Ide::ClaudeCode => home.to_path_buf(),
    }
}

/// Register chert's hooks and MCP server for `ide` in `target`; a stub
/// until src/install T1 lands.
///
/// # Errors
/// None yet.
pub fn install(target: &Target<'_>, ide: Ide) -> Result<Change, Error> {
    let _ = (target, ide);
    Ok(Change::Unchanged)
}

/// Remove only the entries chert wrote; a stub until src/install T1.
///
/// # Errors
/// None yet.
pub fn uninstall(target: &Target<'_>, ide: Ide) -> Result<Change, Error> {
    let _ = (target, ide);
    Ok(Change::Unchanged)
}
