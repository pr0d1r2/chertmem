//! `chert completions <shell>` and `chert man` (root I), generated from
//! the same clap definition as the help text (V1).

#[cfg(test)]
mod tests;

use clap::Subcommand;
use clap_complete::Shell;

use super::{Done, Io};

#[derive(Debug, Clone, Copy, Subcommand)]
pub(super) enum Verb {
    /// A shell completion script on stdout
    Completions { shell: Shell },
    /// The man page (roff) on stdout
    Man,
}

pub(super) fn run(verb: Verb, _io: &mut Io<'_>) -> Done {
    super::not_built(verb)
}
