//! CLI-wide tests and the sandbox every verb test runs in: a temp home
//! and project, never the real `~/.claude*`, config or data dir (T2).

use std::error::Error as StdError;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use super::{Io, run};

pub(super) type Test = Result<(), Box<dyn StdError>>;
pub(super) type Res<T> = Result<T, Box<dyn StdError>>;

/// Milliseconds of 2026-10-24, the time every test verb runs at.
pub(super) const NOW: u64 = 1_792_800_000_000;

/// The `chert` the sandbox wires into IDE configs.
pub(super) const CHERT: &str = "/opt/chert-test/bin/chert";

fn now() -> u64 {
    NOW
}

/// What 1 run of `chert` gave.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Ran {
    pub(super) code: u8,
    pub(super) out: String,
    pub(super) err: String,
}

/// `home/` and `project/` in a fresh temp dir, removed on drop.
pub(super) struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    pub(super) fn new(name: &str) -> Res<Self> {
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("chert-cli-{pid}-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home"))?;
        fs::create_dir_all(dir.join("project"))?;
        Ok(Self { dir })
    }

    pub(super) fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    pub(super) fn project(&self) -> PathBuf {
        self.dir.join("project")
    }

    /// `chert <args>` with empty stdin.
    pub(super) fn run(&self, args: &[&str]) -> Ran {
        let home = self.home();
        self.run_in(args, b"", Some(&home))
    }

    /// `chert <args>` with `stdin`, in the project, `home` injected.
    fn run_in(&self, args: &[&str], stdin: &[u8], home: Option<&Path>) -> Ran {
        let (mut input, project) = (stdin, self.project());
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let mut io = Io {
            stdin: &mut input,
            stdout: &mut out,
            stderr: &mut err,
            cwd: &project,
            home,
            now,
            chert: CHERT,
        };
        let code = run(argv(args), &mut io);
        let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
        let (out, err) = (text(&out), text(&err));
        Ran { code, out, err }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// `chert`, then `args`.
fn argv<'a>(args: &'a [&'a str]) -> impl Iterator<Item = &'a str> {
    std::iter::once("chert").chain(args.iter().copied())
}

#[test]
fn help_is_exit_0_on_stdout() -> Test {
    let s = Sandbox::new("help")?;
    let ran = s.run(&["--help"]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert!(ran.out.contains("Usage: chert"), "{ran:?}");
    Ok(())
}

#[test]
fn version_is_exit_0() -> Test {
    let s = Sandbox::new("version")?;
    let ran = s.run(&["--version"]);
    assert_eq!((ran.code, ran.out.trim()), (0, "chert 0.0.0"));
    Ok(())
}

#[test]
fn no_verb_is_usage_exit_2_on_stderr() -> Test {
    let s = Sandbox::new("no-verb")?;
    let ran = s.run(&[]);
    assert_eq!((ran.code, ran.out.as_str()), (2, ""), "{ran:?}");
    assert!(ran.err.contains("Usage"), "{ran:?}");
    Ok(())
}

#[test]
fn an_unknown_verb_is_exit_2_with_a_suggestion() -> Test {
    let s = Sandbox::new("unknown-verb")?;
    let ran = s.run(&["partition", "list"]);
    assert_eq!(ran.code, 2, "{ran:?}");
    assert!(ran.err.contains("partitions"), "{ran:?}");
    Ok(())
}

#[test]
fn main_takes_a_test_harness_arguments_as_usage() {
    // The harness's own arguments name no verb: clap refuses them before
    // any verb could read the real home, config or data dir.
    let code = super::main();
    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::from(2)));
}

#[test]
fn the_system_clock_is_past_the_test_time() {
    assert!(super::clock() > NOW);
}
