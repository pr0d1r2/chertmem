//! `chert install`, `uninstall`, `status`, `doctor` and `config` (root
//! I): the IDE wiring under the home dir, and `chert.toml`.

#[cfg(test)]
mod tests;

use clap::Subcommand;

use super::args::IdeName;
use super::settings::{self, Config};
use super::{Done, Io};

#[derive(Debug, Subcommand)]
pub(super) enum Verb {
    /// Wire chert's hooks and MCP server into the IDE's config
    Install(Ides),
    /// Remove only chert's entries from the IDE's config
    Uninstall(Ides),
    /// Each of chert's entries in the IDE's config
    Status(Ides),
    /// Check wiring, catalog drift and stale spool; exit 1 on findings
    Doctor(Ides),
    /// `chert.toml`: show, get, set, path
    Config {
        #[command(subcommand)]
        action: Config,
    },
}

#[derive(Debug, clap::Args)]
pub(super) struct Ides {
    /// The IDE to wire
    #[arg(long, value_enum, default_value = "claude-code")]
    ide: IdeName,
}

pub(super) fn run(verb: Verb, io: &mut Io<'_>) -> Done {
    match verb {
        Verb::Config { action } => settings::run(action, io),
        other => super::not_built(other),
    }
}
