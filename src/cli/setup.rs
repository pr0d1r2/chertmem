//! `chert install`, `uninstall`, `status`, `doctor` and `config` (root
//! I): the IDE wiring under the home dir, and `chert.toml`.

#[cfg(test)]
mod tests;

use clap::Subcommand;

use super::args::IdeName;
use super::settings::{self, Config};
use super::{Done, Io, fail, say};
use crate::hook::Ide;
use crate::install::{
    self, Change, Target, WireState, mcp_path, settings_path,
};

#[derive(Debug, Subcommand)]
pub(super) enum Verb {
    /// Wire chert's hooks and MCP server into the IDE's config
    Install(Ides),
    /// Remove only chert's entries from the IDE's config
    Uninstall(Ides),
    /// Each of chert's entries in the IDE's config
    Status(Ides),
    /// Check wiring, catalog drift and stale spool; exit 1 on findings
    Doctor(Ides),
    /// `chert.toml`: show, get, set, path
    Config {
        #[command(subcommand)]
        action: Config,
    },
}

#[derive(Debug, clap::Args)]
pub(super) struct Ides {
    /// The IDE to wire
    #[arg(long, value_enum, default_value = "claude-code")]
    ide: IdeName,
}

/// A verb on 1 IDE's 2 config files.
type Wiring = fn(&Target<'_>, Ide, &mut Io<'_>) -> Done;

pub(super) fn run(verb: Verb, io: &mut Io<'_>) -> Done {
    match verb {
        Verb::Install(a) => wired(a.ide.0, io, install),
        Verb::Uninstall(a) => wired(a.ide.0, io, uninstall),
        Verb::Status(a) => wired(a.ide.0, io, status),
        Verb::Doctor(a) => wired(a.ide.0, io, doctor),
        Verb::Config { action } => settings::run(action, io),
    }
}

/// `then` on `ide`'s files under the home dir.
fn wired(ide: Ide, io: &mut Io<'_>, then: Wiring) -> Done {
    let home = io.home.ok_or_else(no_home)?;
    let (settings, mcp) = (settings_path(home, ide), mcp_path(home, ide));
    let target = Target {
        settings: &settings,
        mcp: &mcp,
        chert: io.chert,
    };
    then(&target, ide, io)
}

pub(super) fn no_home() -> String {
    "chert: no home dir (set HOME)".to_owned()
}

fn install(target: &Target<'_>, ide: Ide, io: &mut Io<'_>) -> Done {
    let changes = install::install(target, ide).map_err(fail)?;
    changed(target, changes, io)
}

fn uninstall(target: &Target<'_>, ide: Ide, io: &mut Io<'_>) -> Done {
    let changes = install::uninstall(target, ide).map_err(fail)?;
    changed(target, changes, io)
}

/// `<file>: wrote|unchanged|restored` for both files.
fn changed(t: &Target<'_>, c: install::Changes, io: &mut Io<'_>) -> Done {
    let word = |c| match c {
        Change::Unchanged => "unchanged",
        Change::Wrote => "wrote",
        Change::Restored => "restored",
    };
    say(
        io,
        &format!("{}: {}", t.settings.display(), word(c.settings)),
    )?;
    say(io, &format!("{}: {}", t.mcp.display(), word(c.mcp)))?;
    Ok(0)
}

/// `<entry>: ok|missing|runs <other>` per entry; exit 0 either way.
fn status(target: &Target<'_>, ide: Ide, io: &mut Io<'_>) -> Done {
    for wire in install::status(target, ide).map_err(fail)?.wires {
        let state = match wire.state {
            WireState::Ok => "ok".to_owned(),
            WireState::Missing => "missing (run chert install)".to_owned(),
            WireState::Other(found) => format!("runs {found}"),
        };
        say(io, &format!("{}: {state}", wire.name))?;
    }
    Ok(0)
}

/// Config warnings, then 1 line per finding: exit 1 when any.
fn doctor(target: &Target<'_>, ide: Ide, io: &mut Io<'_>) -> Done {
    let loaded = crate::config::load_from(io.home, io.cwd).map_err(fail)?;
    for warning in &loaded.warnings {
        say(io, warning)?;
    }
    let report = install::doctor(target, ide, &loaded.config, io.cwd);
    let report = report.map_err(fail)?;
    for finding in &report.findings {
        say(io, &finding.to_string())?;
    }
    if report.findings.is_empty() {
        say(io, "chert: no findings")?;
    }
    Ok(report.exit())
}
