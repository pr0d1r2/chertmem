//! `chert export <out.jsonl> [--for rekall]` and `chert import
//! <in.jsonl> [--from cavemem]` (root I) over `import::{export, jsonl,
//! cavemem}`.

#[cfg(test)]
mod tests;

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use clap::builder::PossibleValue;
use clap::{ArgMatches, ValueEnum, value_parser};

use super::args::{one, option, value, verb};
use super::{Done, Io, fail, load, open, say};
use crate::import;
use crate::store::Filter;

#[derive(Debug)]
pub(super) enum Verb {
    Export {
        out: PathBuf,
        reader: Option<Reader>,
    },
    Import {
        input: PathBuf,
        from: Option<Writer>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Reader {
    Rekall,
}

impl ValueEnum for Reader {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Rekall]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new("rekall"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Writer {
    Cavemem,
}

impl ValueEnum for Writer {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Cavemem]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new("cavemem"))
    }
}

/// `export <OUT> [--for]` and `import <INPUT> [--from]`.
pub(super) fn commands() -> Vec<clap::Command> {
    let reader = option("reader", "READER", value_parser!(Reader));
    let from = option("from", "FROM", value_parser!(Writer));
    let export = verb("export", EXPORT)
        .arg(value("out", "OUT", value_parser!(PathBuf)))
        .arg(reader.long("for").help("The reader's shape"));
    let import = verb("import", IMPORT)
        .arg(value("input", "INPUT", value_parser!(PathBuf)))
        .arg(from.help("What wrote the input"));
    vec![export, import]
}

const EXPORT: &str = "Every observation as JSONL, 1 per line with its ref";
const IMPORT: &str = "Restore a `chert export`, or a cavemem 0.2.1 database";

/// The verb `name` names, when it is 1 of this group's.
pub(super) fn parse(name: &str, m: &ArgMatches) -> Option<Verb> {
    match name {
        "export" => Some(Verb::Export {
            out: one(m, "out")?,
            reader: one(m, "reader"),
        }),
        "import" => Some(Verb::Import {
            input: one(m, "input")?,
            from: one(m, "from"),
        }),
        _ => None,
    }
}

pub(super) fn run(verb: Verb, io: &mut Io<'_>) -> Done {
    match verb {
        Verb::Export {
            reader: Some(Reader::Rekall),
            ..
        } => Err("chert: export --for rekall not built yet (src/import T3)"
            .to_owned()),
        Verb::Export { out, reader: None } => export(&out, io),
        Verb::Import { input, from: None } => jsonl(&input, io),
        Verb::Import {
            input,
            from: Some(Writer::Cavemem),
        } => cavemem(&input, io),
    }
}

/// Write the whole store to `out`.
fn export(out: &Path, io: &mut Io<'_>) -> Done {
    let mut store = open(&load(io)?, io)?;
    let bad = |e: std::io::Error| {
        format!("chert: cannot write {}: {e}", out.display())
    };
    let mut file = BufWriter::new(File::create(out).map_err(bad)?);
    let n = import::export(&mut store, &Filter::default(), &mut file);
    let n = n.map_err(fail)?;
    file.flush().map_err(bad)?;
    say(
        io,
        &format!("exported {n} observations to {}", out.display()),
    )?;
    Ok(0)
}

fn jsonl(input: &Path, io: &mut Io<'_>) -> Done {
    let mut store = open(&load(io)?, io)?;
    let file = File::open(input)
        .map_err(|e| format!("chert: cannot read {}: {e}", input.display()))?;
    let n = import::jsonl(&mut store, BufReader::new(file)).map_err(fail)?;
    say(io, &format!("imported {n} lines from {}", input.display()))?;
    Ok(0)
}

fn cavemem(db: &Path, io: &mut Io<'_>) -> Done {
    let done = import::cavemem(&load(io)?, io.cwd, db).map_err(fail)?;
    let (obs, sums) = (done.observations, done.summaries);
    let line = format!("imported {obs} observations, {sums} summaries");
    let note = format!("{} without a cwd, under this project", done.no_cwd);
    say(io, &format!("{line} ({note})"))?;
    Ok(0)
}
