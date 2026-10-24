//! The tools, params and limits of cavemem 0.2.1's MCP server (V1,
//! root R1), refs in place of its integer ids (root I).
#![cfg_attr(not(test), expect(dead_code, reason = "RED stub (src/mcp T2)"))]

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
}

/// The answer to `tools/list`.
pub(super) fn list() -> Value {
    Value::Null
}

/// The call `params` names; the message says what is wrong.
///
/// # Errors
/// An unknown tool, a missing or bad argument.
pub(super) fn parse(params: &Value) -> Result<Call, String> {
    let _ = (params, SEARCH_MAX, TIMELINE_MAX, IDS_MAX, json!({}));
    let _ = Args::new("", &Value::Null);
    Err(String::new())
}
