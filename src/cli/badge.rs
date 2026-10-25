//! `chert statusline` (root I, T3, R6): an itok-style badge,
//! `chert:<ms>ms,<n>,<size>`, for an IDE's statusline. RED stub: the
//! badge of no data, whatever is stored or running.

#[cfg(test)]
mod tests;

#[allow(dead_code, reason = "RED stub: run does not draw yet")]
mod cells;

use clap::builder::PossibleValue;
use clap::{ArgMatches, ValueEnum};

use super::args::{one, option, verb};
use super::{Done, Io, say};

#[derive(Debug, Clone, Copy)]
pub(super) struct Verb {
    color: When,
}

/// `--color`: when the badge wears ANSI colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum When {
    /// When stdin held a statusline payload or stdout is a terminal.
    Auto,
    Always,
    Never,
}

impl ValueEnum for When {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Auto, Self::Always, Self::Never]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(match self {
            Self::Auto => "auto",
            Self::Always => "always",
            Self::Never => "never",
        }))
    }
}

const ABOUT: &str = "A statusline badge: chert:<ms>ms,<n>,<size>, itok-style";
const COLOR: &str =
    "Color the badge; auto colors a statusline payload or a terminal";

/// `statusline [--color WHEN]`.
pub(super) fn commands() -> Vec<clap::Command> {
    let color = option("color", "WHEN", clap::value_parser!(When));
    let color = color.required(false).default_value("auto").help(COLOR);
    vec![verb("statusline", ABOUT).arg(color)]
}

/// The verb `name` names, when it is this group's.
pub(super) fn parse(name: &str, m: &ArgMatches) -> Option<Verb> {
    let color = (name == "statusline").then(|| one(m, "color"))??;
    Some(Verb { color })
}

/// Print the badge; exit 0 always, as a statusline must never fail.
pub(super) fn run(verb: Verb, io: &mut Io<'_>) -> Done {
    let _ = verb.color;
    say(io, "chert:-ms,0,0B")?;
    Ok(0)
}
