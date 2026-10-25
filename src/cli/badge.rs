//! `chert statusline` (root I, T3, R6): an itok-style badge,
//! `chert:<ms>ms,<n>,<size>`, for an IDE's statusline. It reads the
//! statusline payload, the mcp heartbeat files and the catalog only:
//! no cache, no partition file, no spool replay; and exits 0 whatever
//! is missing, as a statusline must never fail.

#[cfg(test)]
mod tests;

mod cells;

use std::path::{Path, PathBuf};

use clap::builder::PossibleValue;
use clap::{ArgMatches, ValueEnum};
use serde_json::Value;

use super::args::{one, option, verb};
use super::{Done, Io, say};
use crate::cache::{Cache, Plan};
use crate::config::{self, Config};
use crate::mcp::{self, Pulse};
use crate::store::{Filter, Store, project_id};
use cells::{Hue, band, count, fit, ms, paint, size};

#[derive(Debug, Clone, Copy)]
pub(super) struct Verb {
    color: When,
}

/// `--color`: when the badge wears ANSI colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum When {
    /// When stdin held a statusline payload or stdout is a terminal.
    Auto,
    Always,
    Never,
}

impl ValueEnum for When {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Auto, Self::Always, Self::Never]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(match self {
            Self::Auto => "auto",
            Self::Always => "always",
            Self::Never => "never",
        }))
    }
}

const ABOUT: &str = "A statusline badge: chert:<ms>ms,<n>,<size>, itok-style";
const COLOR: &str =
    "Color the badge; auto colors a statusline payload or a terminal";

/// `statusline [--color WHEN]`.
pub(super) fn commands() -> Vec<clap::Command> {
    let color = option("color", "WHEN", clap::value_parser!(When));
    let color = color.required(false).default_value("auto").help(COLOR);
    vec![verb("statusline", ABOUT).arg(color)]
}

/// The verb `name` names, when it is this group's.
pub(super) fn parse(name: &str, m: &ArgMatches) -> Option<Verb> {
    let color = (name == "statusline").then(|| one(m, "color"))??;
    Some(Verb { color })
}

/// Print the badge of the payload's project, else the working dir's.
pub(super) fn run(verb: Verb, io: &mut Io<'_>) -> Done {
    let payload = payload(io);
    let root = payload.as_ref().and_then(project_dir);
    let root = root.unwrap_or_else(|| io.cwd.to_path_buf());
    let color = match verb.color {
        When::Always => true,
        When::Never => false,
        When::Auto => io.tty.stdout || payload.is_some(),
    };
    let badge = Badge::of(io, &root);
    say(io, &badge.render(color))?;
    Ok(0)
}

/// The statusline payload: a JSON object on stdin; `None` when stdin
/// is a terminal (never read: it would wait for input), empty or
/// anything else.
fn payload(io: &mut Io<'_>) -> Option<Value> {
    if io.tty.stdin {
        return None;
    }
    let mut text = Vec::new();
    io.stdin.read_to_end(&mut text).ok()?;
    let value: Value = serde_json::from_slice(&text).ok()?;
    value.is_object().then_some(value)
}

/// The payload's project: `workspace.project_dir`, where the IDE
/// started (and so its `chert mcp`), else `cwd`.
fn project_dir(payload: &Value) -> Option<PathBuf> {
    let dirs = ["/workspace/project_dir", "/cwd"];
    let named = dirs.iter().filter_map(|p| payload.pointer(p)?.as_str());
    named.into_iter().find(|d| !d.is_empty()).map(PathBuf::from)
}

/// What the badge shows; the default is no data at all.
#[derive(Debug, Default)]
struct Badge {
    live: bool,
    /// The median reply in ms and its hue.
    ms: Option<(u64, Hue)>,
    /// Observations and their hue; no hue without a catalog.
    rows: (u64, Option<Hue>),
    /// Stored bytes and their hue; no hue without a catalog.
    bytes: (u64, Option<Hue>),
}

impl Badge {
    /// The badge of the project at `root`; a config that cannot load
    /// names no data dir, so it is the badge of no data.
    fn of(io: &Io<'_>, root: &Path) -> Self {
        let loaded = config::load_from(io.home, root);
        let at = (io.now)();
        loaded.map_or_else(
            |_| Self::default(),
            |l| Self::measure(&l.config, root, at),
        )
    }

    /// The badge of the project at `root` at `now` (ms).
    fn measure(cfg: &Config, root: &Path, now: u64) -> Self {
        let t = &cfg.statusline;
        let dir = cfg.storage.data_dir.as_deref();
        let pulse =
            dir.map_or(Pulse::Dead, |d| mcp::pulse(d, &project_id(root)));
        let median = match pulse {
            Pulse::Live { median_us } => median_us.map(ms),
            Pulse::Dead => None,
        };
        let (green, amber) = (t.ms_green.into(), t.ms_amber.into());
        Self {
            live: pulse != Pulse::Dead,
            ms: median.map(|m| (m, band(m, green, amber))),
            ..Self::stored(stored(cfg, root, now), cfg)
        }
    }

    /// The count and size cells of `stored`, colored by `cfg`.
    fn stored(stored: Option<(u64, u64, Plan)>, cfg: &Config) -> Self {
        let Some((rows, bytes, plan)) = stored else {
            return Self::default();
        };
        let t = &cfg.statusline;
        let fill = fit(plan.estimate, plan.limit, t.fit_amber);
        let heft = band(bytes, t.size_green, t.size_amber);
        Self {
            rows: (rows, Some(fill)),
            bytes: (bytes, Some(heft)),
            ..Self::default()
        }
    }

    /// `chert:<ms>ms,<n>,<size>`, each value painted when `color`.
    fn render(&self, color: bool) -> String {
        let label = if self.live { Hue::Live } else { Hue::Down };
        let label = paint("chert", Some(label), color);
        let text = self
            .ms
            .map_or_else(|| "-".to_owned(), |(m, _)| m.to_string());
        let ms = paint(&format!("{text}ms"), self.ms.map(|(_, h)| h), color);
        let n = paint(&count(self.rows.0), self.rows.1, color);
        let bytes = paint(&size(self.bytes.0), self.bytes.1, color);
        format!("{label}:{ms},{n},{bytes}")
    }
}

/// The project's rows and bytes over every partition the catalog
/// names for it, and the cache window's plan; `None` without a catalog.
fn stored(cfg: &Config, root: &Path, now: u64) -> Option<(u64, u64, Plan)> {
    let store = Store::peek(cfg, root).ok()??;
    let filter = Filter {
        project: Some(store.project().to_owned()),
        ..Filter::default()
    };
    let parts = store.partitions(&filter).ok()?;
    let sum = |f: fn(&crate::store::Stats) -> u64| {
        parts
            .iter()
            .map(|p| f(&p.stats))
            .fold(0, u64::saturating_add)
    };
    let plan = Cache::plan_at(&store, cfg, now).ok()?;
    Some((sum(|s| s.rows), sum(|s| s.bytes), plan))
}
