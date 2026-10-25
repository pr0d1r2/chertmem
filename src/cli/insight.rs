//! `chert mine [--since T] [--format text|json]` and `chert report
//! [--format text|json|md]` (root I) over `mine::{mine, report}`.

#[cfg(test)]
mod tests;

use clap::{Subcommand, ValueEnum};

use super::args::{Since, since};
use super::{Done, Io};

#[derive(Debug, Clone, Copy, Subcommand)]
pub(super) enum Verb {
    /// Behaviour findings with evidence refs; exit 1 when any
    Mine {
        /// Only rows since `<n>h`, `<n>d`, `<n>mo` ago, or ms since epoch
        #[arg(long, value_parser = since)]
        since: Option<Since>,
        #[arg(long, value_enum, default_value = "text")]
        format: MineFormat,
    },
    /// The awareness report: every finding; exit 0
    Report {
        #[arg(long, value_enum, default_value = "text")]
        format: ReportFormat,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum MineFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum ReportFormat {
    Text,
    Json,
    Md,
}

pub(super) fn run(verb: Verb, _io: &mut Io<'_>) -> Done {
    super::not_built(verb)
}
