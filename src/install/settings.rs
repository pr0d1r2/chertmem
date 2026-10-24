//! The IDE config file as bytes and as JSON: load, render, write in
//! place, and the `<file>.chert.bak` backup (src/install C, R1).

#[cfg(test)]
mod tests;

use std::ffi::OsString;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::Error;

/// An IDE config as found: its bytes, `None` when there is no file, and
/// its JSON, an empty object then.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Loaded {
    pub(super) bytes: Option<Vec<u8>>,
    pub(super) value: Value,
}

/// The bytes at `path`, `None` when there is no file.
pub(super) fn bytes(path: &Path) -> Result<Option<Vec<u8>>, Error> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::io(path, &e)),
    }
}

/// The JSON of `bytes`; blank text is an empty object, as a new file.
pub(super) fn parse(path: &Path, bytes: &[u8]) -> Result<Value, Error> {
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return Ok(Value::Object(Map::new()));
    }
    serde_json::from_slice(bytes).map_err(|e| Error::Json {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })
}

/// The config at `path`.
pub(super) fn load(path: &Path) -> Result<Loaded, Error> {
    let bytes = bytes(path)?;
    let value = match &bytes {
        Some(b) => parse(path, b)?,
        None => Value::Object(Map::new()),
    };
    Ok(Loaded { bytes, value })
}

/// `value` as chert writes it: 2-space indent, final newline, as
/// Claude Code and cavemem write it.
pub(super) fn render(value: &Value) -> Vec<u8> {
    let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
    text.push('\n');
    text.into_bytes()
}

/// `<path>.<suffix>`: the name with `suffix` appended.
fn beside(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().map(OsString::from).unwrap_or_default();
    name.push(suffix);
    path.with_file_name(name)
}

/// `<path>.chert.bak`, the original bytes before chert's first write.
#[must_use]
pub(super) fn backup(path: &Path) -> PathBuf {
    beside(path, ".chert.bak")
}

/// `bytes` at `path` whole: written to `<path>.chert.tmp`, then renamed
/// into place, so the IDE never reads half a file.
pub(super) fn write(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let dir = path.parent().unwrap_or(path);
    std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, &e))?;
    let tmp = beside(path, ".chert.tmp");
    std::fs::write(&tmp, bytes).map_err(|e| Error::io(&tmp, &e))?;
    std::fs::rename(&tmp, path).map_err(|e| Error::io(path, &e))
}

/// Keep `original` as the backup unless one is there: the backup holds
/// the bytes before chert's first write, never a later state.
pub(super) fn back_up(path: &Path, original: &[u8]) -> Result<(), Error> {
    let bak = backup(path);
    if bytes(&bak)?.is_some() {
        return Ok(());
    }
    write(&bak, original)
}

/// Remove the file at `path`; one already gone is fine.
pub(super) fn remove(path: &Path) -> Result<(), Error> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(Error::io(path, &e)),
        _ => Ok(()),
    }
}
