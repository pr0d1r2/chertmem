//! The Claude Code dialect: its hook input JSON as a [`Payload`]
//! (src/hook C, D24). Unknown fields are ignored.
//!
//! Read per event: `source` (session-start), `prompt`
//! (user-prompt-submit), `tool_name`, `tool_input`, `tool_response` and
//! `tool_use_id` (post-tool-use), the same with `error` for
//! `tool_response` (post-tool-use-failure), `last_assistant_message`
//! (stop), `reason` (session-end). Only `tool_use_id` is an identity
//! (root V16).

#[cfg(test)]
mod tests;

use serde_json::{Map, Value};

use super::{Kind, Payload};

type Object = Map<String, Value>;

/// The `tool_input` keys that name a path the tool read or wrote.
const PATH_KEYS: [&str; 3] = ["file_path", "path", "notebook_path"];

/// `raw` as a payload of `kind`; `None` when it is not a JSON object
/// with a non-empty `session_id` string.
pub(super) fn parse(kind: Kind, raw: &[u8]) -> Option<Payload> {
    let value: Value = serde_json::from_slice(raw).ok()?;
    let obj = value.as_object()?;
    let session = text(obj, "session_id").filter(|s| !s.is_empty())?;
    Some(Payload {
        session: session.to_owned(),
        identity: identity(kind, obj),
        body: body(kind, obj),
        paths: paths(kind, obj),
        source: text(obj, "source").map(str::to_owned),
    })
}

fn text<'a>(obj: &'a Object, key: &str) -> Option<&'a str> {
    obj.get(key).and_then(Value::as_str)
}

/// Whether `kind` reports 1 tool call, done or failed.
fn calls_tool(kind: Kind) -> bool {
    matches!(kind, Kind::PostToolUse | Kind::PostToolUseFailure)
}

/// `tool_use_id`, the only identity Claude Code documents (root V16).
fn identity(kind: Kind, obj: &Object) -> Option<String> {
    let id = text(obj, "tool_use_id").filter(|id| !id.is_empty());
    id.filter(|_| calls_tool(kind)).map(str::to_owned)
}

fn body(kind: Kind, obj: &Object) -> String {
    let field = |key| text(obj, key).unwrap_or_default().to_owned();
    match kind {
        Kind::SessionStart => field("source"),
        Kind::UserPromptSubmit => field("prompt"),
        Kind::PostToolUse => tool(obj, &json(obj, "tool_response")),
        Kind::PostToolUseFailure => tool(obj, &field("error")),
        Kind::Stop => field("last_assistant_message"),
        Kind::SessionEnd => field("reason"),
    }
}

/// `obj[key]` as compact JSON with sorted keys, so the same bytes every
/// run (V3); empty when absent.
fn json(obj: &Object, key: &str) -> String {
    obj.get(key).map_or_else(String::new, Value::to_string)
}

/// Tool name, input as JSON, then `outcome`, 1 line each: the response
/// as JSON for a done call, the error text for a failed one (T9).
fn tool(obj: &Object, outcome: &str) -> String {
    let name = text(obj, "tool_name").unwrap_or_default();
    let input = json(obj, "tool_input");
    format!("{name}\n{input}\n{outcome}")
}

fn paths(kind: Kind, obj: &Object) -> Vec<String> {
    let input = obj.get("tool_input").and_then(Value::as_object);
    let Some(input) = input.filter(|_| calls_tool(kind)) else {
        return Vec::new();
    };
    let found = PATH_KEYS.iter().filter_map(|key| text(input, key));
    found.map(str::to_owned).collect()
}
