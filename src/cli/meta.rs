//! `chert completions <shell>` and `chert man` (root I), generated from
//! the same clap definition as the help text (V1).

#[cfg(test)]
mod tests;

use clap::{ArgMatches, CommandFactory, value_parser};
use clap_complete::Shell;
use clap_mangen::Man;

use super::args::{Cli, one, value, verb};
use super::{Done, Io, fail};

#[derive(Debug, Clone, Copy)]
pub(super) enum Verb {
    Completions { shell: Shell },
    Man,
}

/// `completions <SHELL>` and `man`.
pub(super) fn commands() -> Vec<clap::Command> {
    let shell = value("shell", "SHELL", value_parser!(Shell));
    vec![
        verb("completions", "A shell completion script on stdout").arg(shell),
        verb("man", "The man page (roff) on stdout"),
    ]
}

/// The verb `name` names, when it is 1 of this group's.
pub(super) fn parse(name: &str, m: &ArgMatches) -> Option<Verb> {
    match name {
        "completions" => {
            one(m, "shell").map(|shell| Verb::Completions { shell })
        }
        "man" => Some(Verb::Man),
        _ => None,
    }
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
