//! IDE wiring (src/install G): chert's hooks and MCP server registered
//! in, and removed from, an IDE's config; `status` and `doctor` read it
//! back. v1 IDE = Claude Code (D24).
//!
//! An IDE keeps hooks and MCP servers in 2 files: for Claude Code
//! `~/.claude/settings.json` and `~/.claude.json` (V4, B1). Every entry
//! takes both as a [`Target`], so tests edit temp files, never a real
//! home's. Each file is edited on its own, by the same rules.
//!
//! Byte identity (V1, R1): the first write keeps the file's original
//! bytes in `<file>.chert.bak`. `uninstall` puts those bytes back, and
//! drops the backup, when what is left after removing chert's entries
//! is the backup's JSON; a file chert created goes again when nothing
//! but chert's entries was in it. Otherwise something else changed in
//! between: the rest is written as JSON and the backup kept.
//!
//! The IDE itself rewrites `~/.claude.json` often, so chert never holds
//! it: it is read and checked first, so an error stops before any
//! write, then read again right before its own write. A rewrite keeps
//! the file's permissions; its backup gets them too.

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
    /// The file the IDE reads hooks from, e.g. [`settings_path`].
    pub settings: &'a Path,
    /// The file the IDE reads MCP servers from, e.g. [`mcp_path`].
    pub mcp: &'a Path,
    /// The `chert` program the hooks and the MCP server run: a name on
    /// `PATH` or a path to the binary.
    pub chert: &'a str,
}

/// What a call did to a config file.
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

/// What a call did to each file of a [`Target`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Changes {
    pub settings: Change,
    pub mcp: Change,
}

/// The file `ide` reads hooks from under home dir `home`.
#[must_use]
pub fn settings_path(home: &Path, ide: Ide) -> PathBuf {
    match ide {
        Ide::ClaudeCode => home.join(".claude").join("settings.json"),
    }
}

/// The file `ide` reads user-scope MCP servers from under home dir
/// `home` (V4).
#[must_use]
pub fn mcp_path(home: &Path, ide: Ide) -> PathBuf {
    match ide {
        Ide::ClaudeCode => home.join(".claude.json"),
    }
}

/// An edit of 1 file's JSON with the `chert` program; `Err` names the
/// wrong-typed key.
type Edit = fn(&mut Value, &str) -> Result<(), String>;

/// The edits of `ide`'s 2 files: install's, then uninstall's.
fn edits(ide: Ide) -> [(Edit, Edit); 2] {
    match ide {
        Ide::ClaudeCode => [
            (claude_code::wire, claude_code::serve),
            (
                |v, _| claude_code::unwire(v),
                |v, _| claude_code::unserve(v),
            ),
        ],
    }
}

/// Register chert's hooks and MCP server for `ide` in `target`; every
/// other key is kept. A file already wired is left untouched (V2); the
/// first write keeps the original as `<file>.chert.bak`.
///
/// # Errors
/// A file is unreadable, not JSON, holds a wrong-typed key chert edits,
/// or cannot be written.
pub fn install(target: &Target<'_>, ide: Ide) -> Result<Changes, Error> {
    let [(hooks, server), _] = edits(ide);
    edited(target.mcp, server, target.chert)?;
    Ok(Changes {
        settings: add(target.settings, hooks, target.chert)?,
        mcp: add(target.mcp, server, target.chert)?,
    })
}

/// Remove only the entries chert wrote for `ide` from `target`, each
/// file back to its original bytes when nothing else changed (V1).
///
/// # Errors
/// As [`install`].
pub fn uninstall(target: &Target<'_>, ide: Ide) -> Result<Changes, Error> {
    let [_, (hooks, server)] = edits(ide);
    edited(target.mcp, server, target.chert)?;
    Ok(Changes {
        settings: take(target.settings, hooks, target.chert)?,
        mcp: take(target.mcp, server, target.chert)?,
    })
}

/// The file at `path` as found, and its JSON after `edit`; `None` when
/// the edit changes nothing.
fn edited(
    path: &Path,
    edit: Edit,
    chert: &str,
) -> Result<Option<(settings::Loaded, Value)>, Error> {
    let found = settings::load(path)?;
    let mut value = found.value.clone();
    edit(&mut value, chert).map_err(|key| Error::Shape {
        path: path.to_path_buf(),
        key,
    })?;
    Ok((value != found.value).then_some((found, value)))
}

/// `edit` applied to `path` and written, the original backed up first.
fn add(path: &Path, edit: Edit, chert: &str) -> Result<Change, Error> {
    let Some((found, value)) = edited(path, edit, chert)? else {
        return Ok(Change::Unchanged);
    };
    if let Some(original) = &found.bytes {
        settings::back_up(path, original)?;
    }
    settings::write(path, &settings::render(&value))?;
    Ok(Change::Wrote)
}

/// `edit` applied to `path`, then put back as the byte-identity rule
/// says.
fn take(path: &Path, edit: Edit, chert: &str) -> Result<Change, Error> {
    match edited(path, edit, chert)? {
        Some((_, value)) => put_back(path, &value),
        None => Ok(Change::Unchanged),
    }
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
