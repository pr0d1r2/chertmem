//! `chert config show|get|set|path` (root I) over `config::{show, get,
//! set, paths}`, home and project injected through [`Io`].

#[cfg(test)]
mod tests;

use clap::Subcommand;

use super::{Done, Io};

#[derive(Debug, Subcommand)]
pub(super) enum Config {
    /// Every key's effective value and the file it came from
    Show,
    /// 1 key's effective value
    Get { key: String },
    /// Set 1 key in the project file, or the user file with --user
    Set {
        key: String,
        /// A TOML value; text that is not one is taken as a string
        value: String,
        /// Write `~/.config/chert/chert.toml` instead
        #[arg(long)]
        user: bool,
    },
    /// The user and project `chert.toml` paths
    Path,
}

pub(super) fn run(action: Config, _io: &mut Io<'_>) -> Done {
    super::not_built(action)
}
