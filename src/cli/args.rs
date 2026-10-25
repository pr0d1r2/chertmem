//! The 1 clap definition of `chert` (src/cli C, V1): help, completions
//! and the man page are all generated from [`Cli`]. Each verb group's
//! own enum lives beside the code that runs it and is flattened here,
//! so every root I verb stays a top-level `chert <verb>`.

#[cfg(test)]
mod tests;

use clap::builder::PossibleValue;
use clap::{Parser, Subcommand, ValueEnum};

use super::{insight, meta, port, query, serve, setup, text, tiers};
use crate::config::Span;
use crate::hook::Ide;

/// Persistent, partitioned memory for coding agents.
#[derive(Debug, Parser)]
#[command(name = "chert", version)]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Command,
}

/// Every verb of root I, by the group that runs it.
#[derive(Debug, Subcommand)]
pub(super) enum Command {
    #[command(flatten)]
    Serve(serve::Verb),
    #[command(flatten)]
    Query(query::Verb),
    #[command(flatten)]
    Setup(setup::Verb),
    #[command(flatten)]
    Tiers(tiers::Verb),
    #[command(flatten)]
    Insight(insight::Verb),
    #[command(flatten)]
    Text(text::Verb),
    #[command(flatten)]
    Port(port::Verb),
    #[command(flatten)]
    Meta(meta::Verb),
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

/// Parse a `--since` value: `<n>h|d|mo` (`config::Span`) or raw ms.
pub(super) fn since(text: &str) -> Result<Since, String> {
    if let Some(span) = Span::parse(text) {
        return Ok(Since::Ago(span));
    }
    text.parse().map(Since::At).map_err(|_| {
        format!("want <n>h, <n>d, <n>mo or ms since the epoch, not {text}")
    })
}
