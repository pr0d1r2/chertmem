//! The tools, params and limits of cavemem 0.2.1's MCP server (V1,
//! root R1), refs in place of its integer ids (root I).

#[cfg(test)]
mod tests;

mod args;

use serde_json::{Value, json};

use crate::store::Ref;
use args::Args;

/// The most hits `search` gives, as cavemem.
const SEARCH_MAX: u32 = 50;
/// The most rows `timeline` gives, as cavemem.
const TIMELINE_MAX: u32 = 200;
/// The most refs 1 `get_observations` takes, as cavemem.
const IDS_MAX: usize = 50;
/// The sessions `list_sessions` gives unless asked, as cavemem.
const SESSIONS_DEFAULT: u32 = 20;
/// The most sessions `list_sessions` gives, as cavemem.
const SESSIONS_MAX: u32 = 200;

/// 1 parsed `tools/call`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Call {
    Search {
        query: String,
        limit: Option<u32>,
    },
    Timeline {
        session_id: String,
        around_id: Option<Ref>,
        limit: Option<u32>,
    },
    Get {
        ids: Vec<Ref>,
        expand: bool,
    },
    Sessions {
        limit: u32,
    },
}

/// The answer to `tools/list`: cavemem's descriptions, but for refs and
/// `expand` defaulting false (root I).
pub(super) fn list() -> Value {
    json!({"tools": [search(), timeline(), get_observations(),
        list_sessions()]})
}

fn search() -> Value {
    let props = json!({"query": text(), "limit": count(SEARCH_MAX)});
    let about = "Search memory. Returns compact hits - fetch full bodies \
                 via get_observations.";
    tool("search", about, &props, &["query"])
}

fn timeline() -> Value {
    let props = json!({"session_id": text(), "around_id": reference(),
        "limit": count(TIMELINE_MAX)});
    let about = "Chronological observation IDs for a session. Use to \
                 locate context around a point.";
    tool("timeline", about, &props, &["session_id"])
}

fn get_observations() -> Value {
    let ids = json!({"type": "array", "items": reference(),
        "minItems": 1, "maxItems": IDS_MAX});
    let props = json!({"ids": ids, "expand": {"type": "boolean"}});
    let about = "Fetch observation bodies by ID. Returns compressed text \
                 unless expand is true.";
    tool("get_observations", about, &props, &["ids"])
}

fn list_sessions() -> Value {
    let props = json!({"limit": count(SESSIONS_MAX)});
    let about = "List recent sessions in reverse chronological order. \
                 Use to navigate before calling timeline.";
    tool("list_sessions", about, &props, &[])
}

fn tool(name: &str, about: &str, props: &Value, required: &[&str]) -> Value {
    json!({"name": name, "description": about, "inputSchema": {
        "type": "object", "properties": props, "required": required,
        "additionalProperties": false}})
}

fn text() -> Value {
    json!({"type": "string", "minLength": 1})
}

fn count(max: u32) -> Value {
    json!({"type": "integer", "exclusiveMinimum": 0, "maximum": max})
}

fn reference() -> Value {
    json!({"type": "string", "pattern": "^chert:[0-9A-HJKMNP-TV-Z]{8,26}$"})
}

/// The call `params` names; the message says what is wrong.
///
/// # Errors
/// An unknown tool, a missing or bad argument.
pub(super) fn parse(params: &Value) -> Result<Call, String> {
    let name = params.get("name").and_then(Value::as_str);
    let name = name.ok_or_else(|| "Invalid params: no tool name".to_owned())?;
    let a = Args::new(name, params.get("arguments").unwrap_or(&Value::Null));
    match name {
        "search" => Ok(Call::Search {
            query: a.text("query")?,
            limit: a.count("limit", SEARCH_MAX)?,
        }),
        "timeline" => around(&a),
        "get_observations" => get(&a),
        "list_sessions" => recent(&a),
        other => Err(format!("Tool {other} not found")),
    }
}

fn around(a: &Args<'_>) -> Result<Call, String> {
    Ok(Call::Timeline {
        session_id: a.text("session_id")?,
        around_id: a.reference("around_id")?,
        limit: a.count("limit", TIMELINE_MAX)?,
    })
}

/// `limit` defaults to 20, as cavemem.
fn recent(a: &Args<'_>) -> Result<Call, String> {
    let limit = a.count("limit", SESSIONS_MAX)?;
    Ok(Call::Sessions {
        limit: limit.unwrap_or(SESSIONS_DEFAULT),
    })
}

/// `expand` defaults false, unlike cavemem (root I).
fn get(a: &Args<'_>) -> Result<Call, String> {
    Ok(Call::Get {
        ids: a.refs("ids", IDS_MAX)?,
        expand: a.flag("expand")?.unwrap_or(false),
    })
}
