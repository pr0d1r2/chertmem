//! `chert hook run <event>` and `chert mcp` (root I): the 2 verbs an
//! IDE runs. `hook run` opens neither the cache nor the search engine:
//! it hands stdin to `hook::run` and exits (root C, 5 ms).

#[cfg(test)]
mod tests;

use clap::builder::PossibleValue;
use clap::{Subcommand, ValueEnum};

use super::args::IdeName;
use super::{Done, Io};
use crate::hook::Kind;

#[derive(Debug, Clone, Copy, Subcommand)]
pub(super) enum Verb {
    /// IDE hook events: JSON on stdin, 1 write, then exit
    Hook {
        #[command(subcommand)]
        action: Hook,
    },
    /// MCP server on stdio, hosting the recent-window cache
    Mcp,
}

#[derive(Debug, Clone, Copy, Subcommand)]
pub(super) enum Hook {
    /// Store 1 event read from stdin; injecting events print context
    Run {
        event: Event,
        /// The IDE whose payload dialect stdin holds
        #[arg(long, value_enum, default_value = "claude-code")]
        ide: IdeName,
    },
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

pub(super) fn run(verb: Verb, _io: &mut Io<'_>) -> Done {
    super::not_built(verb)
}
