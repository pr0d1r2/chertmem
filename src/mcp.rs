//! `chert mcp`: an MCP server on stdio with cavemem 0.2.1's tools,
//! hosting the cache of the project's recent window (src/mcp G).
//!
//! [`serve`] is the library entry `chert mcp` calls; stdin, stdout and
//! the log are injected, so tests drive whole sessions in-process.

#[cfg(test)]
mod tests;

mod error;
mod rpc;
mod server;
mod shape;
mod tools;

use std::io::{BufRead, Write};
use std::path::Path;

pub use error::Error;

use crate::cache::Env;
use crate::config::Config;
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

/// Answer 1 JSON-RPC frame per stdin line on `output` until stdin ends.
///
/// # Errors
/// The store or engine cannot be opened; stdin or stdout fails.
pub fn serve(
    ctx: &Ctx<'_>,
    input: impl BufRead,
    mut output: impl Write,
) -> Result<(), Error> {
    let mut server = Server::open(ctx)?;
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(frame) = server.reply(&line) {
            writeln!(output, "{frame}")?;
            output.flush()?;
        }
    }
    Ok(())
}
