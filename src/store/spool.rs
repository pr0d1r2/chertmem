//! `spool/`: events a lock kept out of their partition, 1 JSONL file per
//! event, written whole then renamed into place, so a reader never sees
//! half a line and a writer never appends to a file being replayed
//! (src/store V10, R2).

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use tinyjson::JsonValue;

use super::write::Row;
use super::{Error, Event};

/// A spool file as a replay finds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Found {
    Rows(Vec<Row>),
    /// Another reader replayed it first.
    Gone,
    /// Not a spool line: set aside under `spool/unreadable/`, kept.
    Unreadable,
}

fn dir(root: &Path) -> PathBuf {
    root.join("spool")
}

/// Spool `row` as its own file.
pub(super) fn put(root: &Path, row: &Row) -> Result<(), Error> {
    let dir = dir(root);
    std::fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, &e))?;
    let name = ulid::Ulid::generate().to_string();
    let tmp = dir.join(format!(".{name}.tmp"));
    let line = format!("{}\n", encode(row)?);
    std::fs::write(&tmp, line).map_err(|e| Error::io(&tmp, &e))?;
    let done = dir.join(format!("{name}.jsonl"));
    std::fs::rename(&tmp, &done).map_err(|e| Error::io(&done, &e))
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

/// The rows of the spool file at `path`.
pub(super) fn read(path: &Path) -> Result<Found, Error> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Found::Gone),
        Err(e) if e.kind() == ErrorKind::InvalidData => String::new(),
        Err(e) => return Err(Error::io(path, &e)),
    };
    let lines = text.lines().filter(|l| !l.trim().is_empty());
    let rows: Option<Vec<Row>> = lines.map(decode).collect();
    Ok(match rows {
        Some(rows) if !rows.is_empty() => Found::Rows(rows),
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
    let aside = dir(root).join("unreadable");
    std::fs::create_dir_all(&aside).map_err(|e| Error::io(&aside, &e))?;
    let to = aside.join(path.file_name().unwrap_or_default());
    match std::fs::rename(path, &to) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(Error::io(&to, &e)),
        _ => Ok(()),
    }
}

/// 1 spool line: every field a string, the timestamp in decimal, so no
/// value passes through a float.
pub(super) fn encode(row: &Row) -> Result<String, Error> {
    let object = fields(row).into_iter().map(|(k, v)| (k.to_owned(), v));
    let value = JsonValue::Object(object.collect::<HashMap<_, _>>());
    value.stringify().map_err(|e| Error::Sqlite {
        reason: format!("spool line: {e}"),
    })
}

fn text(s: &str) -> JsonValue {
    JsonValue::String(s.to_owned())
}

fn fields(row: &Row) -> [(&'static str, JsonValue); 9] {
    let e = &row.event;
    let identity = e.identity.as_deref().map_or(JsonValue::Null, text);
    [
        ("type", text("observation")),
        ("id", text(&row.id)),
        ("project", text(&row.project)),
        ("ts", text(&e.ts.to_string())),
        ("session", text(&e.session)),
        ("kind", text(&e.kind)),
        ("ide", text(&e.ide)),
        ("identity", identity),
        ("body", text(&e.body)),
    ]
}

/// The row of 1 spool line; `None` for anything else.
pub(super) fn decode(line: &str) -> Option<Row> {
    let value: JsonValue = line.parse().ok()?;
    let map: &HashMap<String, JsonValue> = value.get()?;
    if field(map, "type")? != "observation" {
        return None;
    }
    let (id, project) = (field(map, "id")?, field(map, "project")?);
    let event = event(map)?;
    Some(Row { id, project, event })
}

fn field(map: &HashMap<String, JsonValue>, key: &str) -> Option<String> {
    map.get(key)?.get::<String>().cloned()
}

fn event(map: &HashMap<String, JsonValue>) -> Option<Event> {
    let identity = match map.get("identity")? {
        JsonValue::Null => None,
        other => Some(other.get::<String>()?.clone()),
    };
    Some(Event {
        ts: field(map, "ts")?.parse().ok()?,
        session: field(map, "session")?,
        kind: field(map, "kind")?,
        ide: field(map, "ide")?,
        identity,
        body: field(map, "body")?,
    })
}
