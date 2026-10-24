//! Hook intake: 1 IDE event as JSON on stdin, redacted, compressed and
//! appended; injecting events then print context (src/hook G).
//!
//! [`run`] is the library entry `chert hook run <event>` calls; every
//! input it reads is injected through [`Ctx`], so tests use temp dirs.

#[cfg(test)]
mod tests;

mod claude_code;
mod exclude;
mod kind;
mod raw;
mod redact;

use std::path::Path;

pub use kind::{Ide, Kind};

use crate::config::{Config, Grammar};
use crate::grammar::{self, Intensity};
use crate::store::{Event, Mode, Store};

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

impl Outcome {
    /// Done: exit 0, `stdout` printed if any.
    fn done(stdout: Option<String>) -> Self {
        Self {
            stdout,
            exit: 0,
            note: None,
        }
    }

    /// Done all the same (V2), with `why` for stderr.
    fn noted(why: String) -> Self {
        Self {
            note: Some(why),
            ..Self::done(None)
        }
    }
}

/// Handle hook `kind` on payload `stdin` (src/hook G); exit 0 whatever
/// the payload, an unreadable one kept raw (V2).
#[must_use]
pub fn run(kind: Kind, stdin: &[u8], ctx: &Ctx<'_>) -> Outcome {
    let parsed = match ctx.ide {
        Ide::ClaudeCode => claude_code::parse(kind, stdin),
    };
    match parsed {
        Some(payload) => handle(kind, &payload, stdin, ctx),
        None => kept(stdin, ctx, "chert: unreadable hook payload"),
    }
}

/// Store a readable payload; one touching an excluded path stores
/// nothing (V1); one the store refuses is kept raw (V2).
fn handle(
    kind: Kind,
    payload: &Payload,
    stdin: &[u8],
    ctx: &Ctx<'_>,
) -> Outcome {
    let globs = &ctx.config.privacy.exclude;
    if exclude::excluded(globs, ctx.root, &payload.paths) {
        return Outcome::done(None);
    }
    let event = event(kind, payload, ctx);
    let stored = Store::open(ctx.config, ctx.root, Mode::Write)
        .and_then(|mut store| store.append(&event));
    match stored {
        Ok(_) => Outcome::done(None),
        Err(e) => kept(stdin, ctx, &e.to_string()),
    }
}

/// Keep `stdin` raw in the spool, noting `why` and where it went.
fn kept(stdin: &[u8], ctx: &Ctx<'_>, why: &str) -> Outcome {
    Outcome::noted(match raw::keep(ctx.config, stdin) {
        Ok(path) => format!("{why}; kept raw in {}", path.display()),
        Err(e) => format!("{why}; payload lost: {e}"),
    })
}

/// The event `payload` stores, its body redacted and compressed.
fn event(kind: Kind, payload: &Payload, ctx: &Ctx<'_>) -> Event {
    Event {
        ts: ctx.now,
        session: payload.session.clone(),
        kind: kind.as_str().to_owned(),
        ide: ctx.ide.as_str().to_owned(),
        identity: payload.identity.clone(),
        body: prepare(&payload.body, ctx.config),
    }
}

/// `text` as stored: private spans removed, then compressed at
/// `compression.grammar` (V1: redaction comes first).
fn prepare(text: &str, config: &Config) -> String {
    let kept = redact::strip(text);
    grammar::compress(&kept, intensity(config.compression.grammar))
}

fn intensity(grammar: Grammar) -> Intensity {
    match grammar {
        Grammar::Off => Intensity::Off,
        Grammar::Lite => Intensity::Lite,
        Grammar::Full => Intensity::Full,
        Grammar::Ultra => Intensity::Ultra,
    }
}
