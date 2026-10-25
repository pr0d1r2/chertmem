//! Tool results in cavemem 0.2.1's field names (V1, root R1): each a
//! JSON array sent as 1 text content item, as cavemem's server does.

#[cfg(test)]
mod tests;

use serde_json::{Value, json};

use crate::grammar;
use crate::search::{Entry, Hit};
use crate::store::{Lookup, Session};

/// `search`: `[{id, score, snippet, session_id, ts}]`.
pub(super) fn hits(found: &[Hit]) -> Value {
    let hit = |h: &Hit| {
        json!({"id": h.reference.to_string(), "score": h.score,
            "snippet": h.snippet, "session_id": h.session, "ts": h.ts})
    };
    found.iter().map(hit).collect()
}

/// `timeline`: `[{id, kind, ts}]`.
pub(super) fn entries(found: &[Entry]) -> Value {
    let entry = |e: &Entry| {
        let id = e.reference.to_string();
        json!({"id": id, "kind": e.kind, "ts": e.ts})
    };
    found.iter().map(entry).collect()
}

/// `get_observations`: `[{id, session_id, kind, ts, content, metadata}]`
/// in the order asked; `content` compressed as stored unless `expand`
/// (root I); a gone ref answers `{id, dropped: true}` (root V5).
/// chert keeps no metadata, so it is `null`.
pub(super) fn observations(found: &[Lookup], expand: bool) -> Value {
    found.iter().map(|l| lookup(l, expand)).collect()
}

fn lookup(found: &Lookup, expand: bool) -> Value {
    match found {
        Lookup::Found(o) => json!({
            "id": o.reference.to_string(), "session_id": o.session,
            "kind": o.kind, "ts": o.ts, "metadata": null,
            "content": if expand { grammar::expand(&o.body) } else {
                o.body.clone()
            },
        }),
        Lookup::Dropped(r) => json!({"id": r.to_string(), "dropped": true}),
    }
}

/// `list_sessions`: not built yet.
pub(super) fn sessions(found: &[Session]) -> Value {
    found.iter().map(|_| Value::Null).collect()
}

/// A tool's result:`value` as JSON text in 1 content item.
pub(super) fn content(value: &Value) -> Value {
    json!({"content": [{"type": "text", "text": value.to_string()}]})
}

/// A failed tool call, as the SDK answers one: the message as text,
/// `isError` set.
pub(super) fn error(message: &str) -> Value {
    json!({"content": [{"type": "text", "text": message}], "isError": true})
}
