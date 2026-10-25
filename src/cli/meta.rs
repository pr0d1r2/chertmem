//! `chert completions <shell>` and `chert man` (root I), generated from
//! the same clap definition as the help text (V1).

#[cfg(test)]
mod tests;

use clap::{CommandFactory, Subcommand};
use clap_complete::Shell;
use clap_mangen::Man;

use super::args::Cli;
use super::{Done, Io, fail};

#[derive(Debug, Clone, Copy, Subcommand)]
pub(super) enum Verb {
    /// A shell completion script on stdout
    Completions { shell: Shell },
    /// The man page (roff) on stdout
    Man,
}

pub(super) fn run(verb: Verb, io: &mut Io<'_>) -> Done {
    let mut command = Cli::command();
    match verb {
        Verb::Completions { shell } => {
            clap_complete::generate(shell, &mut command, "chert", io.stdout);
        }
        Verb::Man => Man::new(command).render(io.stdout).map_err(fail)?,
    }
    Ok(0)
}
