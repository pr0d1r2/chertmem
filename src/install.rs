//! IDE wiring (src/install G): chert's hooks and MCP server registered
//! in, and removed from, an IDE's config; `status` and `doctor` read it
//! back. v1 IDE = Claude Code (D24).
//!
//! Every entry takes the config file as [`Target::settings`], so tests
//! edit temp files, never a real `~/.claude/settings.json`.
//!
//! Byte identity (V1, R1): the first write keeps the file's original
//! bytes in `<file>.chert.bak`. `uninstall` puts those bytes back, and
//! drops the backup, when what is left after removing chert's entries
//! is the backup's JSON; a file chert created goes again when nothing
//! but chert's entries was in it. Otherwise something else changed in
//! between: the rest is written as JSON and the backup kept.

#[cfg(test)]
mod tests;

mod claude_code;
mod doctor;
mod error;
mod settings;

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

pub use doctor::{Drift, Finding, Report, Status, Wire, WireState};
pub use doctor::{doctor, status};
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
        Ide::ClaudeCode => home.join(".claude").join("settings.json"),
    }
}

/// The key a wrong-typed value sits under, as an error on `path`.
fn shaped(path: &Path, edited: Result<(), String>) -> Result<(), Error> {
    edited.map_err(|key| Error::Shape {
        path: path.to_path_buf(),
        key,
    })
}

/// Register chert's hooks and MCP server for `ide` in `target`; every
/// other key is kept. A file already wired is left untouched (V2); the
/// first write keeps the original as `<file>.chert.bak`.
///
/// # Errors
/// The file is unreadable, not JSON, holds a wrong-typed key chert
/// edits, or cannot be written.
pub fn install(target: &Target<'_>, ide: Ide) -> Result<Change, Error> {
    let found = settings::load(target.settings)?;
    let mut value = found.value.clone();
    let wired = match ide {
        Ide::ClaudeCode => claude_code::wire(&mut value, target.chert),
    };
    shaped(target.settings, wired)?;
    if value == found.value {
        return Ok(Change::Unchanged);
    }
    if let Some(original) = &found.bytes {
        settings::back_up(target.settings, original)?;
    }
    settings::write(target.settings, &settings::render(&value))?;
    Ok(Change::Wrote)
}

/// Remove only the entries chert wrote for `ide` from `target`, back to
/// the original bytes when nothing else changed (V1).
///
/// # Errors
/// As [`install`].
pub fn uninstall(target: &Target<'_>, ide: Ide) -> Result<Change, Error> {
    let found = settings::load(target.settings)?;
    let mut value = found.value.clone();
    let unwired = match ide {
        Ide::ClaudeCode => claude_code::unwire(&mut value),
    };
    shaped(target.settings, unwired)?;
    if value == found.value {
        return Ok(Change::Unchanged);
    }
    put_back(target.settings, &value)
}

/// Write `value`, what is left once chert's entries are gone, as the
/// byte-identity rule says (module doc).
fn put_back(path: &Path, value: &Value) -> Result<Change, Error> {
    let bak = settings::backup(path);
    let created = value.as_object().is_some_and(Map::is_empty);
    match settings::bytes(&bak)? {
        Some(b) if settings::parse(&bak, &b).ok().as_ref() == Some(value) => {
            settings::write(path, &b)?;
            settings::remove(&bak)?;
        }
        None if created => settings::remove(path)?,
        _ => {
            settings::write(path, &settings::render(value))?;
            return Ok(Change::Wrote);
        }
    }
    Ok(Change::Restored)
}
