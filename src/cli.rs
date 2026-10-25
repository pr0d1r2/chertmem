//! `chert`: the clap front end (src/cli G). Every verb of root I is
//! parsed by 1 clap definition (`args`), handed to the module that
//! owns it, and its result mapped to exit 0 ok, 1 finding or 2 usage or
//! unreadable input (V2); nothing here decides anything else (C).
//!
//! [`run`] takes every input through [`Io`], so tests drive whole verbs
//! in-process with temp dirs; [`main`] is the system entry `fn main`
//! calls.

#[cfg(test)]
mod tests;

mod args;
mod badge;
mod insight;
mod meta;
mod port;
mod query;
mod serve;
mod settings;
mod setup;
mod text;
mod tiers;

use std::ffi::OsString;
use std::fmt::Display;
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;

use crate::config::{self, Config};
use crate::store::{Mode, Store};
use args::{Cli, Command};

/// What a verb reads and writes besides its arguments.
pub struct Io<'a> {
    pub stdin: &'a mut dyn BufRead,
    pub stdout: &'a mut dyn Write,
    pub stderr: &'a mut dyn Write,
    /// The project root: the working directory.
    pub cwd: &'a Path,
    /// The home dir, for `chert.toml`, the data dir and IDE configs;
    /// `None` skips the user scope.
    pub home: Option<&'a Path>,
    /// Milliseconds since the Unix epoch.
    pub now: fn() -> u64,
    /// Which of stdin and stdout are terminals.
    pub tty: Tty,
    /// The `chert` program `install` wires: a path or a name on `PATH`.
    pub chert: &'a str,
}

/// Which standard streams are terminals: `chert statusline` reads no
/// terminal stdin and colors a terminal stdout (T3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tty {
    pub stdin: bool,
    pub stdout: bool,
}

/// A verb's exit code, or the message of an exit 2 (V2).
type Done = Result<u8, String>;

/// Parse `args` (program name first) and run the verb: its exit code.
pub fn run<I, T>(args: I, io: &mut Io<'_>) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let code = match Cli::try_parse_from(args) {
        Ok(cli) => finish(verb(cli.command, io), io),
        Err(e) => refused(&e, io),
    };
    let _ = io.stdout.flush();
    code
}

/// The system entry: real stdio, working dir, `$HOME` and clock.
#[must_use]
pub fn main() -> ExitCode {
    let cwd = std::env::current_dir().unwrap_or_default();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let exe = std::env::current_exe().ok();
    let (mut stdin, mut stdout) = (std::io::stdin().lock(), std::io::stdout());
    let mut io = Io {
        stdin: &mut stdin,
        stdout: &mut stdout,
        stderr: &mut std::io::stderr(),
        cwd: &cwd,
        home: home.as_deref(),
        now: clock,
        chert: exe.as_deref().and_then(Path::to_str).unwrap_or("chert"),
        tty: Tty::system(),
    };
    ExitCode::from(run(std::env::args_os(), &mut io))
}

impl Tty {
    /// The process's own stdin and stdout.
    fn system() -> Self {
        Self {
            stdin: std::io::stdin().is_terminal(),
            stdout: std::io::stdout().is_terminal(),
        }
    }
}

fn clock() -> u64 {
    let since = SystemTime::now().duration_since(UNIX_EPOCH);
    u64::try_from(since.unwrap_or_default().as_millis()).unwrap_or(u64::MAX)
}

fn verb(command: Command, io: &mut Io<'_>) -> Done {
    match command {
        Command::Serve(v) => serve::run(v, io),
        Command::Query(v) => query::run(v, io),
        Command::Setup(v) => setup::run(v, io),
        Command::Tiers(v) => tiers::run(v, io),
        Command::Insight(v) => insight::run(v, io),
        Command::Badge(v) => badge::run(v, io),
        Command::Text(v) => text::run(v, io),
        Command::Port(v) => port::run(v, io),
        Command::Meta(v) => meta::run(v, io),
    }
}

/// The exit code; a failure's message on stderr, exit 2.
fn finish(done: Done, io: &mut Io<'_>) -> u8 {
    done.unwrap_or_else(|why| {
        let _ = writeln!(io.stderr, "{why}");
        2
    })
}

/// A clap refusal, help or version: printed where clap says, with
/// clap's exit code (2 for usage, 0 for help and version).
fn refused(e: &clap::Error, io: &mut Io<'_>) -> u8 {
    let text = e.render().to_string();
    let sink = if e.use_stderr() {
        &mut *io.stderr
    } else {
        &mut *io.stdout
    };
    let _ = write!(sink, "{text}");
    u8::try_from(e.exit_code()).unwrap_or(2)
}

/// A library error as its message.
fn fail<E: Display>(e: E) -> String {
    e.to_string()
}

/// The merged `chert.toml` of the home and project scope.
fn load(io: &Io<'_>) -> Result<Config, String> {
    let loaded = config::load_from(io.home, io.cwd).map_err(fail)?;
    Ok(loaded.config)
}

/// The project's store, opened for reading.
fn open(config: &Config, io: &Io<'_>) -> Result<Store, String> {
    Store::open(config, io.cwd, Mode::Read).map_err(fail)
}

/// `text` and a newline on stdout.
fn say(io: &mut Io<'_>, text: &str) -> Result<(), String> {
    writeln!(io.stdout, "{text}").map_err(fail)
}

/// `text` on stdout as it is.
fn put(io: &mut Io<'_>, text: &str) -> Result<(), String> {
    write!(io.stdout, "{text}").map_err(fail)
}
