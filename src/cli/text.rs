//! `chert compress <file>` and `chert expand <file>` (root I): the
//! caveman grammar on a file, the result on stdout.

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use clap::{ArgMatches, value_parser};

use super::args::{one, value, verb};
use super::{Done, Io, load, put};
use crate::grammar;

#[derive(Debug)]
pub(super) enum Verb {
    Compress { file: PathBuf },
    Expand { file: PathBuf },
}

/// `compress <FILE>` and `expand <FILE>`.
pub(super) fn commands() -> Vec<clap::Command> {
    let file = || value("file", "FILE", value_parser!(PathBuf));
    vec![
        verb(
            "compress",
            "A file in caveman grammar at `compression.grammar`",
        )
        .arg(file()),
        verb("expand", "A file with every abbreviation written out again")
            .arg(file()),
    ]
}

/// The verb `name` names, when it is 1 of this group's.
pub(super) fn parse(name: &str, m: &ArgMatches) -> Option<Verb> {
    let file = || one(m, "file");
    match name {
        "compress" => file().map(|file| Verb::Compress { file }),
        "expand" => file().map(|file| Verb::Expand { file }),
        _ => None,
    }
}

pub(super) fn run(verb: Verb, io: &mut Io<'_>) -> Done {
    let out = match verb {
        Verb::Compress { file } => {
            let intensity = load(io)?.compression.grammar.intensity();
            grammar::compress(&read(&file)?, intensity)
        }
        Verb::Expand { file } => grammar::expand(&read(&file)?),
    };
    put(io, &out)?;
    Ok(0)
}

/// The file's text; unreadable is exit 2.
pub(super) fn read(file: &Path) -> Result<String, String> {
    std::fs::read_to_string(file)
        .map_err(|e| format!("chert: cannot read {}: {e}", file.display()))
}
