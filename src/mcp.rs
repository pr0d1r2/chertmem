//! `chert mcp`: an MCP server on stdio with cavemem 0.2.1's tools,
//! hosting the cache of the project's recent window (src/mcp G).
//!
//! [`serve`] is the library entry `chert mcp` calls; stdin, stdout and
//! the log are injected, so tests drive whole sessions in-process.

#[cfg(test)]
mod tests;

mod beat;
mod error;
mod rpc;
mod server;
mod shape;
mod tools;

use std::io::{BufRead, Write};
use std::path::Path;
use std::time::Instant;

pub use beat::{Pulse, pulse};
pub use error::Error;

use crate::cache::Env;
use crate::config::Config;
use beat::Beat;
use server::Server;

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

/// Answer 1 JSON-RPC frame per stdin line on `output` until stdin ends,
/// keeping the heartbeat `chert statusline` reads (T4): a reply's
/// latency, from its line read to its frame flushed, is noted after
/// the flush, so the write never delays the reply it measures.
///
/// # Errors
/// The store or engine cannot be opened; stdin or stdout fails.
pub fn serve(
    ctx: &Ctx<'_>,
    input: impl BufRead,
    mut output: impl Write,
) -> Result<(), Error> {
    let mut server = Server::open(ctx)?;
    let mut beat = heartbeat(ctx);
    for line in input.lines() {
        let (line, start) = (line?, Instant::now());
        if let Some(frame) = answer(&mut server, &line) {
            writeln!(output, "{frame}")?;
            output.flush()?;
            let now = (ctx.env.now)();
            let _ = beat.as_mut().map(|b| b.record(start.elapsed(), now));
        }
    }
    Ok(())
}

/// The frame answering `line`; a blank line is skipped.
fn answer(server: &mut Server, line: &str) -> Option<serde_json::Value> {
    if line.trim().is_empty() {
        return None;
    }
    server.reply(line)
}

/// This server's heartbeat; without one it still serves, logged once,
/// and `chert statusline` reads it as down.
fn heartbeat(ctx: &Ctx<'_>) -> Option<Beat> {
    let dir = ctx.config.storage.data_dir.as_deref()?;
    let project = crate::store::project_id(ctx.root);
    let started = Beat::start(dir, &project, std::process::id());
    started
        .map_err(|e| {
            (ctx.env.log)(&format!(
                "chert: mcp heartbeat off: {e} (check the data dir)"
            ));
        })
        .ok()
}
