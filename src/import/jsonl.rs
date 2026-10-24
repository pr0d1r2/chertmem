//! JSONL import: each line of a `chert export` read back into its
//! observation and restored under its own id, ts, project and ref
//! (src/import V1, src/store `restore`).

#[cfg(test)]
mod tests;

use std::io::BufRead;

use serde_json::Value;

use super::Error;
use crate::store::{Observation, Ref, Store};

/// Restore every line of `input`, an export, into `store`, which needs a
/// reading open; the number of lines read, blank ones skipped. A line
/// whose row is already stored adds nothing, so importing twice adds
/// nothing (V1).
///
/// # Errors
/// [`Error::Line`] naming the first line that is not an export line or
/// that the store refuses, e.g. a ref another id holds.
pub fn jsonl(store: &mut Store, input: impl BufRead) -> Result<usize, Error> {
    let mut count = 0_usize;
    for (at, read) in input.lines().enumerate() {
        let line = at.saturating_add(1);
        let restored = read
            .map_err(|e| bad(&e.to_string()))
            .and_then(|t| restore(store, &t))
            .map_err(|e| Error::Line {
                line,
                cause: Box::new(e),
            })?;
        count = count.saturating_add(usize::from(restored));
    }
    Ok(count)
}

/// Restore 1 line; `false` for a blank one.
fn restore(store: &mut Store, text: &str) -> Result<bool, Error> {
    if text.trim().is_empty() {
        return Ok(false);
    }
    store.restore(&observation(text)?)?;
    Ok(true)
}

fn bad(reason: &str) -> Error {
    Error::Jsonl {
        reason: reason.to_owned(),
    }
}

/// 1 export line as the observation it was, keys as `export` writes.
fn observation(text: &str) -> Result<Observation, Error> {
    let value: Value =
        serde_json::from_str(text).map_err(|e| bad(&e.to_string()))?;
    let field = |key: &str| string(&value, key);
    let ts = value.get("ts").and_then(Value::as_u64);
    Ok(Observation {
        reference: Ref::parse(&field("ref")?)?,
        id: field("id")?,
        project: field("project")?,
        ts: ts.ok_or_else(|| missing("ts"))?,
        session: field("session")?,
        ide: field("ide")?,
        kind: field("kind")?,
        identity: identity(&value)?,
        body: field("body")?,
    })
}

/// Text field `key` of `value`.
fn string(value: &Value, key: &str) -> Result<String, Error> {
    let text = value.get(key).and_then(Value::as_str);
    text.map(str::to_owned).ok_or_else(|| missing(key))
}

/// `identity`: text, or null for none.
fn identity(value: &Value) -> Result<Option<String>, Error> {
    match value.get("identity") {
        Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        _ => Err(missing("identity")),
    }
}

fn missing(key: &str) -> Error {
    bad(&format!("no `{key}` of the right type"))
}
