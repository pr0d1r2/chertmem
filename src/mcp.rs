//! `chert mcp`: an MCP server on stdio with cavemem 0.2.1's 4 tools,
//! hosting the cache of the project's recent window (src/mcp G).
//!
//! [`serve`] is the library entry `chert mcp` calls; stdin, stdout and
//! the log are injected, so tests drive whole sessions in-process.

#[cfg(test)]
mod tests;

mod error;
mod rpc;

use std::io::{BufRead, Write};
use std::path::Path;

pub use error::Error;

use serde_json::{Value, json};

use crate::cache::Env;
use crate::config::Config;
use rpc::Frame;

/// What [`serve`] reads besides stdin.
#[derive(Clone)]
pub struct Ctx<'a> {
    pub config: &'a Config,
    /// The project root: the cache holds this project's window.
    pub root: &'a Path,
    /// The clock, memory figure and log; every log line goes to stderr
    /// in `chert mcp`, never stdout (V2) nor a tool result (V3).
    pub env: Env,
}

/// Answer 1 JSON-RPC frame per stdin line on `output` until stdin ends.
///
/// # Errors
/// The store or engine cannot be opened; stdin or stdout fails.
pub fn serve(
    ctx: &Ctx<'_>,
    input: impl BufRead,
    mut output: impl Write,
) -> Result<(), Error> {
    let _ = ctx;
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(frame) = reply(&line) {
            writeln!(output, "{frame}")?;
            output.flush()?;
        }
    }
    Ok(())
}

/// The frame answering `line`, if any.
fn reply(line: &str) -> Option<Value> {
    match rpc::parse(line) {
        Frame::Request { id, method, params } => {
            Some(answer(id, &method, &params))
        }
        Frame::Bad(frame) => Some(frame),
        Frame::Silent => None,
    }
}

fn answer(id: Value, method: &str, params: &Value) -> Value {
    match method {
        "initialize" => rpc::ok(id, rpc::initialize(params)),
        "ping" => rpc::ok(id, json!({})),
        _ => rpc::fail(id, rpc::NO_METHOD, "Method not found"),
    }
}
