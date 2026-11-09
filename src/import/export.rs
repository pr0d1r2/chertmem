//! JSONL export: 1 observation per line, its `ref` as `chert:<ref>`,
//! its body exactly as stored, so nothing redacted can come back (root
//! I, V8).

#[cfg(test)]
mod tests;

use std::io::Write;

use serde_json::{Map, Value};

use super::Error;
use crate::hook::Kind;
use crate::store::{Filter, Observation, Store};

/// Write every observation `filter` wants to `out`, 1 JSON object per
/// line in the store's (ts, id) order; the number of lines.
///
/// # Errors
/// A store that cannot be read (it needs a reading open), or a failed
/// write.
pub fn export(
    store: &mut Store,
    filter: &Filter,
    out: &mut impl Write,
) -> Result<usize, Error> {
    written(&store.observations(filter)?, out)
}

/// The export rekall's `catch` reads (src/import T3, rekall V52): the
/// same lines as [`export`], only `user-prompt-submit` rows, as only a
/// prompt holds a human's words; the number of lines.
///
/// # Errors
/// As [`export`].
pub fn rekall(
    store: &mut Store,
    filter: &Filter,
    out: &mut impl Write,
) -> Result<usize, Error> {
    let mut rows = store.observations(filter)?;
    rows.retain(|o| o.kind == Kind::UserPromptSubmit.as_str());
    written(&rows, out)
}

/// `rows` to `out`, 1 [`line()`] each; their number.
fn written(rows: &[Observation], out: &mut impl Write) -> Result<usize, Error> {
    for row in rows {
        writeln!(out, "{}", line(row)).map_err(|e| Error::io(&e))?;
    }
    out.flush().map_err(|e| Error::io(&e))?;
    Ok(rows.len())
}

/// 1 observation as 1 JSON line, keys sorted (a `BTreeMap`), so the same
/// rows give the same bytes; the body goes out as stored, nothing added.
pub(super) fn line(o: &Observation) -> String {
    let fields = [
        ("ref", Value::from(o.reference.to_string())),
        ("id", Value::from(o.id.as_str())),
        ("project", Value::from(o.project.as_str())),
        ("ts", Value::from(o.ts)),
        ("session", Value::from(o.session.as_str())),
        ("ide", Value::from(o.ide.as_str())),
        ("kind", Value::from(o.kind.as_str())),
        ("identity", Value::from(o.identity.clone())),
        ("body", Value::from(o.body.as_str())),
    ];
    let map: Map<String, Value> =
        fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect();
    Value::Object(map).to_string()
}
