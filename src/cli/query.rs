//! `chert search`, `chert timeline` and `chert get` (root I): the MCP
//! tools' answers on the command line, 1 row per line.

#[cfg(test)]
mod tests;

use clap::{Args, Subcommand};

use super::args::{Since, since};
use super::{Done, Io};

#[derive(Debug, Subcommand)]
pub(super) enum Verb {
    /// Ranked search across partitions, best first
    Search(Search),
    /// A session's rows in order, around 1 ref
    Timeline {
        /// The IDE's session id
        session: String,
        /// Centre on this `chert:<ref>`
        #[arg(long)]
        around: Option<String>,
    },
    /// Observations by `chert:<ref>`; exit 1 when one is dropped
    Get {
        #[arg(required = true)]
        refs: Vec<String>,
    },
}

#[derive(Debug, Args)]
pub(super) struct Search {
    /// The query; several words are joined by spaces
    #[arg(required = true)]
    query: Vec<String>,
    /// Hits at most; default `search.default_limit`
    #[arg(long)]
    limit: Option<u32>,
    /// Only rows since `<n>h`, `<n>d`, `<n>mo` ago, or ms since epoch
    #[arg(long, value_parser = since)]
    since: Option<Since>,
    /// Only this project id, as `chert partitions list` prints it
    #[arg(long)]
    project: Option<String>,
    /// Keyword ranking only, even when embeddings are on
    #[arg(long)]
    no_semantic: bool,
}

pub(super) fn run(verb: Verb, _io: &mut Io<'_>) -> Done {
    super::not_built(verb)
}
