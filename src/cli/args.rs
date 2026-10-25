//! The 1 clap definition of `chert` (src/cli C, V1): help, completions
//! and the man page are all generated from [`Cli`]'s `clap::Command`.
//! It is built with clap's builder API, not its derive, so no proc
//! macro (and no Unicode-3.0 licence) enters the build. Each verb
//! group's subcommands and their parse live beside the code that runs
//! them, chained here, so every root I verb is a top-level `chert
//! <verb>`.

#[cfg(test)]
mod tests;

use clap::builder::{IntoResettable, PossibleValue, ValueParser};
use clap::error::ErrorKind;
use clap::{
    Arg, ArgAction, ArgMatches, CommandFactory, FromArgMatches, Parser,
    ValueEnum,
};

use super::{insight, meta, port, query, serve, setup, text, tiers};
use crate::config::{Span, Unit};
use crate::hook::Ide;

/// Persistent, partitioned memory for coding agents.
#[derive(Debug)]
pub(super) struct Cli {
    pub(super) command: Command,
}

impl CommandFactory for Cli {
    fn command() -> clap::Command {
        clap::Command::new("chert")
            .about("Persistent, partitioned memory for coding agents")
            .version(env!("CARGO_PKG_VERSION"))
            .subcommand_required(true)
            .arg_required_else_help(true)
            .subcommands(verbs())
    }

    fn command_for_update() -> clap::Command {
        Self::command()
    }
}

impl FromArgMatches for Cli {
    fn from_arg_matches(m: &ArgMatches) -> Result<Self, clap::Error> {
        let parsed = m.subcommand().and_then(|(n, sub)| Command::parse(n, sub));
        let unknown = || clap::Error::new(ErrorKind::MissingSubcommand);
        Ok(Self {
            command: parsed.ok_or_else(unknown)?,
        })
    }

    fn update_from_arg_matches(
        &mut self,
        m: &ArgMatches,
    ) -> Result<(), clap::Error> {
        *self = Self::from_arg_matches(m)?;
        Ok(())
    }
}

impl Parser for Cli {}

/// Every verb of root I, in the order help lists them.
fn verbs() -> impl Iterator<Item = clap::Command> {
    let groups = [
        serve::commands(),
        query::commands(),
        setup::commands(),
        tiers::commands(),
        insight::commands(),
        text::commands(),
        port::commands(),
        meta::commands(),
    ];
    groups.into_iter().flatten()
}

/// Every verb of root I, by the group that runs it.
#[derive(Debug)]
pub(super) enum Command {
    Serve(serve::Verb),
    Query(query::Verb),
    Setup(setup::Verb),
    Tiers(tiers::Verb),
    Insight(insight::Verb),
    Text(text::Verb),
    Port(port::Verb),
    Meta(meta::Verb),
}

impl Command {
    /// The verb `name` with its matches, from the group that owns it.
    fn parse(name: &str, m: &ArgMatches) -> Option<Self> {
        let groups: [Group; 8] = [
            |n, m| serve::parse(n, m).map(Self::Serve),
            |n, m| query::parse(n, m).map(Self::Query),
            |n, m| setup::parse(n, m).map(Self::Setup),
            |n, m| tiers::parse(n, m).map(Self::Tiers),
            |n, m| insight::parse(n, m).map(Self::Insight),
            |n, m| text::parse(n, m).map(Self::Text),
            |n, m| port::parse(n, m).map(Self::Port),
            |n, m| meta::parse(n, m).map(Self::Meta),
        ];
        groups.into_iter().find_map(|group| group(name, m))
    }
}

/// 1 verb group's parse of a subcommand it may own.
type Group = fn(&str, &ArgMatches) -> Option<Command>;

/// A verb, its 1-line help as its about.
pub(super) fn verb(name: &'static str, about: &'static str) -> clap::Command {
    clap::Command::new(name).about(about)
}

/// A verb that takes 1 of its actions: help when none is given.
pub(super) fn parent(name: &'static str, about: &'static str) -> clap::Command {
    verb(name, about)
        .subcommand_required(true)
        .arg_required_else_help(true)
}

/// A required positional `<NAME>`, 1 value read by `parser`.
pub(super) fn value(
    id: &'static str,
    name: &'static str,
    parser: impl IntoResettable<ValueParser>,
) -> Arg {
    let arg = Arg::new(id).value_name(name).required(true);
    arg.value_parser(parser).action(ArgAction::Set)
}

/// An optional `--<id> <NAME>`, 1 value read by `parser`.
pub(super) fn option(
    id: &'static str,
    name: &'static str,
    parser: impl IntoResettable<ValueParser>,
) -> Arg {
    let arg = Arg::new(id).value_name(name).long(id);
    arg.value_parser(parser).action(ArgAction::Set)
}

/// A `--<long>` switch; its id and value name are as clap derive named
/// them, so the man page's synopsis stays as it was.
pub(super) fn flag(
    id: &'static str,
    name: &'static str,
    long: &'static str,
) -> Arg {
    let arg = Arg::new(id).value_name(name).required(false).long(long);
    arg.value_parser(clap::value_parser!(bool))
        .action(ArgAction::SetTrue)
}

/// `--ide <IDE>`, `claude-code` when not given.
pub(super) fn ide(help: &'static str) -> Arg {
    let arg = option("ide", "IDE", clap::value_parser!(IdeName));
    arg.required(false).help(help).default_value("claude-code")
}

/// The value of `id` as its parser made it; `None` when not given.
pub(super) fn one<T: Clone + Send + Sync + 'static>(
    m: &ArgMatches,
    id: &str,
) -> Option<T> {
    m.get_one::<T>(id).cloned()
}

/// Every value of `id`, in order.
pub(super) fn many(m: &ArgMatches, id: &str) -> Vec<String> {
    let values = m.get_many::<String>(id).into_iter().flatten();
    values.cloned().collect()
}

/// An `--ide` value: an IDE whose dialect chert reads (D24).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct IdeName(pub(super) Ide);

impl ValueEnum for IdeName {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self(Ide::ClaudeCode)]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(self.0.as_str()))
    }
}

/// A `--since` value: a span back from now, or a time in ms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Since {
    /// `<n>h`, `<n>d` or `<n>mo` before now.
    Ago(Span),
    /// Milliseconds since the Unix epoch.
    At(u64),
}

const HOUR: u64 = 3_600_000;
const DAY: u64 = 86_400_000;
/// A month as the cache counts it: 31 days, so `mo` errs long.
const MONTH: u64 = 2_678_400_000;

impl Since {
    /// The time this names, in ms, at `now`.
    pub(super) fn ms(self, now: u64) -> u64 {
        match self {
            Self::At(ms) => ms,
            Self::Ago(span) => now.saturating_sub(span_ms(span)),
        }
    }
}

fn span_ms(span: Span) -> u64 {
    let unit = match span.unit {
        Unit::Hours => HOUR,
        Unit::Days => DAY,
        Unit::Months => MONTH,
    };
    u64::from(span.amount).saturating_mul(unit)
}

/// `--since`: only rows since a span ago or a time in ms.
pub(super) fn since_arg() -> Arg {
    option("since", "SINCE", since)
        .help("Only rows since `<n>h`, `<n>d`, `<n>mo` ago, or ms since epoch")
}

/// Parse a `--since` value: `<n>h|d|mo` (`config::Span`) or raw ms.
pub(super) fn since(text: &str) -> Result<Since, String> {
    if let Some(span) = Span::parse(text) {
        return Ok(Since::Ago(span));
    }
    text.parse().map(Since::At).map_err(|_| {
        format!("want <n>h, <n>d, <n>mo or ms since the epoch, not {text}")
    })
}
