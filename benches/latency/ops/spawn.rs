//! The hook-write suite: a real `chert hook run post-tool-use` process
//! per call (benches V4), so its startup is counted: exec, config load,
//! store open, 1 write, exit. The process reads `$HOME`'s `chert.toml`,
//! set to a copy of the fixture's data dir, and runs in a project dir
//! of its own: a real one, as a process needs a working dir that
//! exists, so its project starts empty beside the fixture's. Its clock
//! is the system's, so it writes into this week's partition, as a real
//! hook does.

#[cfg(test)]
mod tests;

use std::fs;
use std::hint::black_box;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use chertmem::config::Config;
use chertmem::store::{Filter, Mode, Store};

use crate::fixture::{Fixture, Res, Scratch};
use crate::suites::Op;

/// The `chert` this package builds.
const CHERT: &str = env!("CARGO_BIN_EXE_chert");

/// A Claude Code `PostToolUse` payload: a write, no injection.
pub(crate) const PAYLOAD: &[u8] = br#"{"session_id":"bench","cwd":"/work/project","hook_event_name":"PostToolUse","tool_name":"Bash","tool_input":{"command":"cargo test"},"tool_response":{"stdout":"test result: ok. 12 passed"}}"#;

/// A `chert` to spawn: its home, its project and a copy of a fixture's
/// data, each removed on drop.
#[derive(Debug)]
pub(crate) struct Process {
    home: Scratch,
    project: PathBuf,
    cfg: Config,
    _copy: Scratch,
}

impl Process {
    /// A copy of `fx`'s data, and a home whose `chert.toml` names it.
    pub(crate) fn of(fx: &Fixture) -> Res<Self> {
        let name = format!("hook-write-{}", fx.spec.key());
        let (copy, cfg) = Scratch::of(fx, &name)?;
        let home = Scratch::new(&format!("{name}-home"))?;
        let project = home.path().join("project");
        fs::create_dir_all(&project)?;
        user_file(home.path(), copy.path())?;
        Ok(Self {
            home,
            project,
            cfg,
            _copy: copy,
        })
    }

    /// 1 `chert hook run post-tool-use` of [`PAYLOAD`].
    pub(crate) fn run(&self) -> Res<()> {
        self.run_with(PAYLOAD)
    }

    /// 1 `chert hook run post-tool-use` of `stdin`; a failed exit or a
    /// note on stderr is the error, as no row was stored.
    pub(crate) fn run_with(&self, stdin: &[u8]) -> Res<()> {
        let mut child = self.command().spawn()?;
        child.stdin.take().ok_or("no stdin")?.write_all(stdin)?;
        let out = child.wait_with_output()?;
        let note = String::from_utf8_lossy(&out.stderr);
        if !out.status.success() || !note.is_empty() {
            return Err(
                format!("chert hook run: {}: {note}", out.status).into()
            );
        }
        Ok(())
    }

    /// `chert hook run post-tool-use` in the project, `$HOME` the only
    /// variable, stdin piped, a note caught.
    fn command(&self) -> Command {
        let mut cmd = Command::new(CHERT);
        cmd.args(["hook", "run", "post-tool-use"])
            .current_dir(&self.project)
            .env_clear()
            .env("HOME", self.home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        cmd
    }

    /// The rows the process's project holds.
    pub(crate) fn rows(&self) -> Res<usize> {
        let mut store = Store::open(&self.cfg, &self.project, Mode::Read)?;
        let only = Filter {
            project: Some(store.project().to_owned()),
            ..Filter::default()
        };
        Ok(store.observations(&only)?.len())
    }
}

/// `home`'s `chert.toml`, naming `data` as the data dir.
fn user_file(home: &Path, data: &Path) -> Res<()> {
    let dir = home.join(".config/chert");
    fs::create_dir_all(&dir)?;
    let data = data.to_str().ok_or("data dir is not UTF-8")?;
    let value = toml::Value::String(data.to_owned());
    fs::write(
        dir.join("chert.toml"),
        format!("[storage]\ndata_dir = {value}\n"),
    )?;
    Ok(())
}

/// The hook-write suite on a copy of `fx`: it must store its 1 row
/// once before it is measured, and every measured run is checked.
pub(crate) fn write(fx: &Fixture) -> Res<Op> {
    let process = Process::of(fx)?;
    process.run()?;
    if process.rows()? != 1 {
        return Err("chert hook run stored no row".into());
    }
    Ok(Box::new(move || {
        process.run()?;
        black_box(&process);
        Ok(())
    }))
}
