//! JSON-RPC 2.0 frames, 1 per line on stdio, as the MCP stdio
//! transport sends them (src/mcp T1, R1).

#[cfg(test)]
mod tests;

use serde_json::{Map, Value, json};

/// Not JSON.
pub(super) const PARSE: i64 = -32700;
/// JSON, but not a request.
pub(super) const INVALID: i64 = -32600;
/// A method this server does not have.
pub(super) const NO_METHOD: i64 = -32601;

/// The newest protocol version, as `@modelcontextprotocol/sdk` 1.32.2
/// (what cavemem 0.2.1's `^1.0.0` resolves to) answers with.
pub(super) const LATEST: &str = "2025-11-25";

/// 1 line read from stdin.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Frame {
    Request {
        id: Value,
        method: String,
        params: Value,
    },
    /// Malformed; answered with this error frame.
    Bad(Value),
    /// A notification or a client's response: never answered.
    Silent,
}

/// Every version the SDK accepts, newest first; asked for one of these,
/// the server answers with it.
const SUPPORTED: [&str; 5] = [
    LATEST,
    "2025-06-18",
    "2025-03-26",
    "2024-11-05",
    "2024-10-07",
];

/// Read 1 line as a frame.
pub(super) fn parse(line: &str) -> Frame {
    match serde_json::from_str::<Value>(line) {
        Err(_) => Frame::Bad(fail(Value::Null, PARSE, "Parse error")),
        Ok(Value::Object(map)) => request(map),
        Ok(_) => invalid(Value::Null),
    }
}

/// A request has a string or number id, `jsonrpc` 2.0 and a method;
/// no id, or a result or error, is never answered.
fn request(mut map: Map<String, Value>) -> Frame {
    let reply = map.contains_key("result") || map.contains_key("error");
    let Some(id) = map.remove("id").filter(|_| !reply) else {
        return Frame::Silent;
    };
    if !(id.is_string() || id.is_number()) {
        return invalid(Value::Null);
    }
    call(id, map)
}

/// A request's method and params, under `jsonrpc` 2.0.
fn call(id: Value, mut map: Map<String, Value>) -> Frame {
    let two = map.get("jsonrpc").and_then(Value::as_str) == Some("2.0");
    match map.remove("method") {
        Some(Value::String(method)) if two => Frame::Request {
            id,
            method,
            params: map.remove("params").unwrap_or(Value::Null),
        },
        _ => invalid(id),
    }
}

fn invalid(id: Value) -> Frame {
    Frame::Bad(fail(id, INVALID, "Invalid Request"))
}

/// The answer to `initialize`: the asked version when supported, else
/// the latest, as the SDK negotiates.
pub(super) fn initialize(params: &Value) -> Value {
    let asked = params.get("protocolVersion").and_then(Value::as_str);
    let version = asked.filter(|v| SUPPORTED.contains(v)).unwrap_or(LATEST);
    json!({
        "protocolVersion": version,
        "capabilities": {"tools": {}},
        "serverInfo": {"name": "chert", "version": env!("CARGO_PKG_VERSION")},
    })
}

/// A result frame.
pub(super) fn ok(id: Value, result: Value) -> Value {
    frame(id, "result", result)
}

/// An error frame.
pub(super) fn fail(id: Value, code: i64, message: &str) -> Value {
    frame(id, "error", json!({"code": code, "message": message}))
}

fn frame(id: Value, key: &str, body: Value) -> Value {
    let mut map = Map::new();
    map.insert("jsonrpc".into(), Value::from("2.0"));
    map.insert("id".into(), id);
    map.insert(key.into(), body);
    Value::Object(map)
}
