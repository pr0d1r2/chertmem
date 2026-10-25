//! `spool/`: what a lock kept out of its partition, 1 JSONL file per
//! item, written whole then renamed into place, so a reader never sees
//! half a line and a writer never appends to a file being replayed
//! (src/store V10, R2).

#[cfg(test)]
mod tests;

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::Error;
use super::line;
use super::write::Item;

/// A spool file as a replay finds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Found {
    Items(Vec<Item>),
    /// Another reader replayed it first.
    Gone,
    /// Not a spool line: set aside under `spool/unreadable/`, kept.
    Unreadable,
}

fn dir(root: &Path) -> PathBuf {
    root.join("spool")
}

/// Write `bytes` whole as `<dir>/<ULID>.<ext>`: to `.<ULID>.tmp`
/// first, then renamed into place; the file written.
fn place(dir: &Path, ext: &str, bytes: &[u8]) -> Result<PathBuf, Error> {
    std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, &e))?;
    let name = ulid::Ulid::generate().to_string();
    let tmp = dir.join(format!(".{name}.tmp"));
    std::fs::write(&tmp, bytes).map_err(|e| Error::io(&tmp, &e))?;
    let done = dir.join(format!("{name}.{ext}"));
    std::fs::rename(&tmp, &done).map_err(|e| Error::io(&done, &e))?;
    Ok(done)
}

/// Spool `item` as its own file.
pub(super) fn put(root: &Path, item: &Item) -> Result<(), Error> {
    let text = format!("{}\n", line::encode(item)?);
    place(&dir(root), "jsonl", text.as_bytes()).map(|_| ())
}

/// Keep a hook payload that is no spool line, raw, as
/// `spool/unreadable/<ULID>.hook`, the name `src/hook` writes; no
/// replay lists that dir (src/store T14).
pub(super) fn raw(root: &Path, bytes: &[u8]) -> Result<PathBuf, Error> {
    place(&dir(root).join("unreadable"), "hook", bytes)
}

/// Every spool file, oldest first (ULID names sort by time).
pub(super) fn pending(root: &Path) -> Result<Vec<PathBuf>, Error> {
    let dir = dir(root);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(&dir, &e)),
    };
    let mut files: Vec<PathBuf> = (entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .collect();
    files.sort();
    Ok(files)
}

/// The items of the spool file at `path`.
pub(super) fn read(path: &Path) -> Result<Found, Error> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Found::Gone),
        Err(e) if e.kind() == ErrorKind::InvalidData => String::new(),
        Err(e) => return Err(Error::io(path, &e)),
    };
    let lines = text.lines().filter(|l| !l.trim().is_empty());
    let items: Option<Vec<Item>> = lines.map(line::decode).collect();
    Ok(match items {
        Some(items) if !items.is_empty() => Found::Items(items),
        _ => Found::Unreadable,
    })
}

/// Drop a replayed file; one another reader dropped first is fine.
pub(super) fn done(path: &Path) -> Result<(), Error> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(Error::io(path, &e)),
        _ => Ok(()),
    }
}

/// Move an unreadable file to `spool/unreadable/`, out of every replay.
pub(super) fn set_aside(root: &Path, path: &Path) -> Result<(), Error> {
    move_to(root, path, "unreadable")
}

/// Move a file holding an item for a file sealed past a lost catalog to
/// `spool/sealed/`, kept whole and out of every replay (src/store T10).
pub(super) fn hold(root: &Path, path: &Path) -> Result<(), Error> {
    move_to(root, path, "sealed")
}

fn move_to(root: &Path, path: &Path, under: &str) -> Result<(), Error> {
    let aside = dir(root).join(under);
    std::fs::create_dir_all(&aside).map_err(|e| Error::io(&aside, &e))?;
    let to = aside.join(path.file_name().unwrap_or_default());
    match std::fs::rename(path, &to) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(Error::io(&to, &e)),
        _ => Ok(()),
    }
}
