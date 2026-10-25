//! `chert mine [--since T] [--format text|json]` and `chert report
//! [--format text|json|md]` (root I) over `mine::{mine, report}`.

#[cfg(test)]
mod tests;

use clap::builder::{IntoResettable, PossibleValue, ValueParser};
use clap::{Arg, ArgMatches, ValueEnum, value_parser};

use super::args::{Since, one, option, since_arg, verb};
use super::{Done, Io, fail, load, open, put};
use crate::mine::{self, Format};

#[derive(Debug, Clone, Copy)]
pub(super) enum Verb {
    Mine {
        since: Option<Since>,
        format: MineFormat,
    },
    Report {
        format: ReportFormat,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MineFormat {
    Text,
    Json,
}

impl ValueEnum for MineFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Text, Self::Json]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(match self {
            Self::Text => "text",
            Self::Json => "json",
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReportFormat {
    Text,
    Json,
    Md,
}

impl ValueEnum for ReportFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Text, Self::Json, Self::Md]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::Md => "md",
        }))
    }
}

/// `mine [--since] [--format]` and `report [--format]`.
pub(super) fn commands() -> Vec<clap::Command> {
    let mine = verb(
        "mine",
        "Behaviour findings with evidence refs; exit 1 when any",
    )
    .arg(since_arg())
    .arg(format(value_parser!(MineFormat)));
    let report = verb("report", "The awareness report: every finding; exit 0")
        .arg(format(value_parser!(ReportFormat)));
    vec![mine, report]
}

/// `--format <FORMAT>`, `text` when not given.
fn format(parser: impl IntoResettable<ValueParser>) -> Arg {
    let arg = option("format", "FORMAT", parser).required(false);
    arg.default_value("text")
}

/// The verb `name` names, when it is 1 of this group's.
pub(super) fn parse(name: &str, m: &ArgMatches) -> Option<Verb> {
    match name {
        "mine" => Some(Verb::Mine {
            since: one(m, "since"),
            format: one(m, "format")?,
        }),
        "report" => one(m, "format").map(|format| Verb::Report { format }),
        _ => None,
    }
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
