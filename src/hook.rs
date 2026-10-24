//! Hook intake: 1 IDE event as JSON on stdin, redacted, compressed and
//! appended; injecting events then print context (src/hook G).
//!
//! [`run`] is the library entry `chert hook run <event>` calls; every
//! input it reads is injected through [`Ctx`], so tests use temp dirs.

#[cfg(test)]
mod tests;

mod claude_code;
mod kind;

use std::path::Path;

pub use kind::{Ide, Kind};

use crate::config::Config;

/// What [`run`] reads besides stdin.
#[derive(Debug, Clone, Copy)]
pub struct Ctx<'a> {
    pub config: &'a Config,
    /// The project root: the hook's working directory, which Claude Code
    /// sets to the project.
    pub root: &'a Path,
    /// Milliseconds since the Unix epoch: the event's own time.
    pub now: u64,
    pub ide: Ide,
}

/// What the CLI does once [`run`] is done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Printed on stdout: injected context, or nothing.
    pub stdout: Option<String>,
    /// The exit code: 0 for every payload (V2).
    pub exit: u8,
    /// What went wrong, for stderr only; never the agent's context.
    pub note: Option<String>,
}

/// 1 event as a dialect reads it, before redaction.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Payload {
    session: String,
    /// The identity the dialect supplies (root V16).
    identity: Option<String>,
    /// The text stored for the event.
    body: String,
    /// The paths the event read or wrote, checked by `privacy.exclude`.
    paths: Vec<String>,
    /// `session-start` only: why the session started.
    source: Option<String>,
}

/// Handle hook `kind` on payload `stdin` (src/hook G).
#[must_use]
pub fn run(kind: Kind, stdin: &[u8], ctx: &Ctx<'_>) -> Outcome {
    let _ = (kind, stdin, ctx, claude_code::parse);
    Outcome {
        stdout: None,
        exit: 0,
        note: None,
    }
}
