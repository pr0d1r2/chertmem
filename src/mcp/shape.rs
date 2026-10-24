//! Tool results in cavemem 0.2.1's field names (V1, root R1): each a
//! JSON array sent as 1 text content item, as cavemem's server does.

#[cfg(test)]
mod tests;

use serde_json::{Value, json};

use crate::grammar;
use crate::search::{Entry, Hit};
use crate::store::Lookup;

/// `search`: `[{id, score, snippet, session_id, ts}]`.
pub(super) fn hits(found: &[Hit]) -> Value {
    let _ = found;
    Value::Null
}

/// `timeline`: `[{id, kind, ts}]`.
pub(super) fn entries(found: &[Entry]) -> Value {
    let _ = found;
    Value::Null
}

/// `get_observations`: `[{id, session_id, kind, ts, content, metadata}]`
/// in the order asked; `content` compressed as stored unless `expand`
/// (root I); a gone ref answers `{id, dropped: true}` (root V5).
pub(super) fn observations(found: &[Lookup], expand: bool) -> Value {
    let _ = (found, expand, grammar::expand(""));
    Value::Null
}

/// A tool's result: `value` as JSON text in 1 content item.
pub(super) fn content(value: &Value) -> Value {
    json!({"content": [{"type": "text", "text": value.to_string()}]})
}

/// A failed tool call, as the SDK answers one: the message as text,
/// `isError` set.
pub(super) fn error(message: &str) -> Value {
    json!({"content": [{"type": "text", "text": message}], "isError": true})
}
