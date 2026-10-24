//! JSON-RPC 2.0 frames, 1 per line on stdio, as the MCP stdio
//! transport sends them (src/mcp T1, R1).
#![expect(dead_code, reason = "RED stub (src/mcp T1): serve wires it")]

#[cfg(test)]
mod tests;

use serde_json::{Map, Value, json};

/// Not JSON.
pub(super) const PARSE: i64 = -32700;
/// JSON, but not a request.
pub(super) const INVALID: i64 = -32600;
/// A method this server does not have.
pub(super) const NO_METHOD: i64 = -32601;
/// A method's params are wrong.
pub(super) const PARAMS: i64 = -32602;

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

/// Read 1 line as a frame.
pub(super) fn parse(_line: &str) -> Frame {
    Frame::Silent
}

/// The answer to `initialize`.
pub(super) fn initialize(_params: &Value) -> Value {
    Value::Null
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
