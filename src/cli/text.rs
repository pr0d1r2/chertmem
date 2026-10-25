//! `chert compress <file>` and `chert expand <file>` (root I): the
//! caveman grammar on a file, the result on stdout.

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use clap::Subcommand;

use super::{Done, Io, load, put};
use crate::grammar;

#[derive(Debug, Subcommand)]
pub(super) enum Verb {
    /// A file in caveman grammar at `compression.grammar`
    Compress { file: PathBuf },
    /// A file with every abbreviation written out again
    Expand { file: PathBuf },
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
