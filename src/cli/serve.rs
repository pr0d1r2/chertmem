//! `chert hook run <event>` and `chert mcp` (root I): the 2 verbs an
//! IDE runs. `hook run` opens neither the cache nor the search engine:
//! it hands stdin to `hook::run` and exits (root C, 5 ms).

#[cfg(test)]
mod tests;

use clap::builder::PossibleValue;
use clap::{ArgMatches, ValueEnum};

use super::args::{IdeName, ide, one, parent, value, verb};
use super::{Done, Io, fail, load, put};
use crate::cache::Env;
use crate::config::Config;
use crate::hook::{self, Kind};
use crate::mcp;

#[derive(Debug, Clone, Copy)]
pub(super) enum Verb {
    /// `hook <action>`.
    Hook {
        action: Hook,
    },
    Mcp,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Hook {
    Run { event: Event, ide: IdeName },
}

/// `hook run <EVENT> [--ide]` and `mcp`.
pub(super) fn commands() -> Vec<clap::Command> {
    let run = verb("run", RUN)
        .arg(value("event", "EVENT", clap::value_parser!(Event)))
        .arg(ide("The IDE whose payload dialect stdin holds"));
    let hook = parent("hook", HOOK).subcommand(run);
    vec![hook, verb("mcp", MCP)]
}

const HOOK: &str = "IDE hook events: JSON on stdin, 1 write, then exit";
const RUN: &str =
    "Store 1 event read from stdin; injecting events print context";
const MCP: &str = "MCP server on stdio, hosting the recent-window cache";

/// The verb `name` names, when it is 1 of this group's.
pub(super) fn parse(name: &str, m: &ArgMatches) -> Option<Verb> {
    match name {
        "hook" => {
            let run = m.subcommand_matches("run")?;
            let (event, ide) = (one(run, "event")?, one(run, "ide")?);
            Some(Verb::Hook {
                action: Hook::Run { event, ide },
            })
        }
        "mcp" => Some(Verb::Mcp),
        _ => None,
    }
}

/// A hook event as `hook::Kind` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Event(pub(super) Kind);

const EVENTS: [Event; 6] = [
    Event(Kind::SessionStart),
    Event(Kind::UserPromptSubmit),
    Event(Kind::PostToolUse),
    Event(Kind::PostToolUseFailure),
    Event(Kind::Stop),
    Event(Kind::SessionEnd),
];

impl ValueEnum for Event {
    fn value_variants<'a>() -> &'a [Self] {
        &EVENTS
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(self.0.as_str()))
    }
}

pub(super) fn run(verb: Verb, io: &mut Io<'_>) -> Done {
    match verb {
        Verb::Hook {
            action: Hook::Run { event, ide },
        } => run_hook(event.0, ide, io),
        Verb::Mcp => serve_mcp(io),
    }
}

/// Exit as `hook::run` says. A config or stdin that cannot be read is
/// noted on stderr with exit 0 too: Claude Code takes exit 2 from a
/// hook as blocking, which would stop the agent (src/cli R1).
fn run_hook(kind: Kind, ide: IdeName, io: &mut Io<'_>) -> Done {
    let (stdin, config) = match intake(io) {
        Ok(read) => read,
        Err(why) => return noted(io, &format!("{why}; event not stored")),
    };
    let (root, now) = (io.cwd, (io.now)());
    let ctx = hook::Ctx {
        config: &config,
        root,
        now,
        ide: ide.0,
    };
    let out = hook::run(kind, &stdin, &ctx);
    put(io, out.stdout.as_deref().unwrap_or_default())?;
    out.note
        .map_or(Ok(out.exit), |why| noted(io, &why).map(|_| out.exit))
}

/// The whole of stdin and the config.
fn intake(io: &mut Io<'_>) -> Result<(Vec<u8>, Config), String> {
    let mut stdin = Vec::new();
    io.stdin.read_to_end(&mut stdin).map_err(fail)?;
    Ok((stdin, load(io)?))
}

/// `why` on stderr; exit 0.
fn noted(io: &mut Io<'_>, why: &str) -> Done {
    writeln!(io.stderr, "{why}").map_err(fail)?;
    Ok(0)
}

/// Serve until stdin ends, the cache on the injected clock.
fn serve_mcp(io: &mut Io<'_>) -> Done {
    let config = load(io)?;
    let env = Env {
        now: io.now,
        ..Env::system()
    };
    let ctx = mcp::Ctx {
        config: &config,
        root: io.cwd,
        env,
    };
    mcp::serve(&ctx, &mut *io.stdin, &mut *io.stdout).map_err(fail)?;
    Ok(0)
}
