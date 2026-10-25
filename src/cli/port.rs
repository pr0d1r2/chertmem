//! `chert export <out.jsonl> [--for rekall]` and `chert import
//! <in.jsonl> [--from cavemem]` (root I) over `import::{export, jsonl,
//! cavemem}`.

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use clap::{Subcommand, ValueEnum};

use super::{Done, Io};

#[derive(Debug, Subcommand)]
pub(super) enum Verb {
    /// Every observation as JSONL, 1 per line with its ref
    Export {
        out: PathBuf,
        /// The reader's shape
        #[arg(long = "for", value_enum)]
        reader: Option<Reader>,
    },
    /// Restore a `chert export`, or a cavemem 0.2.1 database
    Import {
        input: PathBuf,
        /// What wrote the input
        #[arg(long, value_enum)]
        from: Option<Writer>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum Reader {
    Rekall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum Writer {
    Cavemem,
}

pub(super) fn run(verb: Verb, _io: &mut Io<'_>) -> Done {
    super::not_built(verb)
}
