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
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;

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
    /// The `chert` program `install` wires: a path or a name on `PATH`.
    pub chert: &'a str,
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
    let chert = exe.as_deref().and_then(Path::to_str).unwrap_or("chert");
    let (mut stdin, mut stdout) = (std::io::stdin().lock(), std::io::stdout());
    let mut io = Io {
        stdin: &mut stdin,
        stdout: &mut stdout,
        stderr: &mut std::io::stderr(),
        cwd: &cwd,
        home: home.as_deref(),
        now: clock,
        chert,
    };
    ExitCode::from(run(std::env::args_os(), &mut io))
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

/// What every verb answers until it is wired (src/cli T1, T2).
fn not_built<T>(verb: T) -> Done {
    drop(verb);
    Err("chert: not built yet (src/cli T2)".to_owned())
}
