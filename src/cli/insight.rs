//! `chert mine [--since T] [--format text|json]` and `chert report
//! [--format text|json|md]` (root I) over `mine::{mine, report}`.

#[cfg(test)]
mod tests;

use clap::{Subcommand, ValueEnum};

use super::args::{Since, since};
use super::{Done, Io, fail, load, open, put};
use crate::mine::{self, Format};

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

pub(super) fn run(verb: Verb, io: &mut Io<'_>) -> Done {
    match verb {
        Verb::Mine { since, format } => {
            let since = since.map(|s| s.ms((io.now)()));
            let format = match format {
                MineFormat::Text => Format::Text,
                MineFormat::Json => Format::Json,
            };
            mined(since, format, io).map(|found| u8::from(found > 0))
        }
        Verb::Report { format } => mined(None, report(format), io).map(|_| 0),
    }
}

fn report(format: ReportFormat) -> Format {
    match format {
        ReportFormat::Text => Format::Text,
        ReportFormat::Json => Format::Json,
        ReportFormat::Md => Format::Md,
    }
}

/// Mine the project's rows since `since`, print them as `format`: the
/// number of findings.
fn mined(
    since: Option<u64>,
    format: Format,
    io: &mut Io<'_>,
) -> Result<usize, String> {
    let config = load(io)?;
    let mut store = open(&config, io)?;
    let found = mine::mine(&mut store, &config, since).map_err(fail)?;
    put(io, &mine::report(&found, format))?;
    Ok(found.len())
}
