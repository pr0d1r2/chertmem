//! `chert compress <file>` and `chert expand <file>` (root I): the
//! caveman grammar on a file, the result on stdout.

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use clap::Subcommand;

use super::{Done, Io};

#[derive(Debug, Subcommand)]
pub(super) enum Verb {
    /// A file in caveman grammar at `compression.grammar`
    Compress { file: PathBuf },
    /// A file with every abbreviation written out again
    Expand { file: PathBuf },
}

pub(super) fn run(verb: Verb, _io: &mut Io<'_>) -> Done {
    super::not_built(verb)
}
