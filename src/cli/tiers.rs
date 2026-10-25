//! `chert partitions [list|seal|compact|drop|repartition]` and `chert
//! cache [plan]` (root I): the store's partitions and the cache's fit.

#[cfg(test)]
mod tests;

use clap::{ArgMatches, value_parser};

use super::args::{one, value, verb};
use super::{Done, Io, fail, load, open, say};
use crate::cache::Cache;
use crate::config::Config;
use crate::store::{Filter, Partition, Store};

#[derive(Debug)]
pub(super) enum Verb {
    /// `partitions`; `list` when no action is given.
    Partitions { action: Partitions },
    /// `cache`; `plan` is its 1 action.
    Cache,
}

#[derive(Debug)]
pub(super) enum Partitions {
    List,
    Seal { path: String },
    Compact { path: String },
    Drop { path: String },
    Repartition,
}

/// The `partitions` actions that take a `<PATH>`.
const ON_PATH: [(&str, &str); 3] = [
    ("seal", "Seal a partition: warm, refusing appends"),
    ("compact", "Seal, then zstd a partition into the cold tier"),
    ("drop", "Drop a partition; its refs answer `dropped`"),
];

/// `partitions [list|seal|compact|drop|repartition]` and `cache [plan]`.
pub(super) fn commands() -> Vec<clap::Command> {
    let on_path = ON_PATH.into_iter().map(|(name, about)| {
        verb(name, about).arg(value("path", "PATH", value_parser!(String)))
    });
    let list = verb("list", LIST);
    let repartition = verb("repartition", REPARTITION);
    let actions = [list].into_iter().chain(on_path).chain([repartition]);
    vec![
        optional("partitions", PARTITIONS).subcommands(actions),
        optional("cache", CACHE).subcommand(verb("plan", PLAN)),
    ]
}

const PARTITIONS: &str =
    "The store's partitions; `list` when no action is given";
const LIST: &str =
    "Every partition: path, tier, rows, bytes, time span, project";
const REPARTITION: &str =
    "Rewrite partitions written under another `partition.*` spec";
const CACHE: &str = "The recent-window cache; `plan` when no action is given";
const PLAN: &str =
    "The window's estimate against `cache.max_bytes`; exit 1 when over";

/// A verb whose action may be left out.
fn optional(name: &'static str, about: &'static str) -> clap::Command {
    verb(name, about)
        .subcommand_required(false)
        .arg_required_else_help(false)
}

/// The verb `name` names, when it is 1 of this group's.
pub(super) fn parse(name: &str, m: &ArgMatches) -> Option<Verb> {
    match name {
        "partitions" => {
            let action = m.subcommand().map_or(Some(Partitions::List), action);
            action.map(|action| Verb::Partitions { action })
        }
        "cache" => Some(Verb::Cache),
        _ => None,
    }
}

/// The `partitions` action `name` names.
fn action((name, m): (&str, &ArgMatches)) -> Option<Partitions> {
    let path = || one::<String>(m, "path");
    match name {
        "list" => Some(Partitions::List),
        "seal" => path().map(|path| Partitions::Seal { path }),
        "compact" => path().map(|path| Partitions::Compact { path }),
        "drop" => path().map(|path| Partitions::Drop { path }),
        "repartition" => Some(Partitions::Repartition),
        _ => None,
    }
}

pub(super) fn run(verb: Verb, io: &mut Io<'_>) -> Done {
    let config = load(io)?;
    let store = open(&config, io)?;
    match verb {
        Verb::Partitions { action } => partitions(store, &action, io),
        Verb::Cache => plan(&store, &config, io),
    }
}

/// A seal or compaction of a bucket not yet ended is exit 2 naming its
/// end (`src/store:V13`).
fn partitions(mut store: Store, action: &Partitions, io: &mut Io<'_>) -> Done {
    let now = (io.now)();
    let done = match action {
        Partitions::List => return list(&store, io),
        Partitions::Seal { path } => store.seal_at(path, now),
        Partitions::Compact { path } => store.compact_at(path, now),
        Partitions::Drop { path } => store.drop(path),
        Partitions::Repartition => store.repartition(),
    };
    done.map_err(fail)?;
    Ok(0)
}

/// `<path> <tier> <rows> rows <bytes> B <t0>..<t1> <project>`.
fn list(store: &Store, io: &mut Io<'_>) -> Done {
    for p in store.partitions(&Filter::default()).map_err(fail)? {
        say(io, &row(&p))?;
    }
    Ok(0)
}

fn row(p: &Partition) -> String {
    let (rows, bytes) = (p.stats.rows, p.stats.bytes);
    let tier = p.state.as_str();
    let span = format!("{}..{}", p.t0, p.t1);
    format!(
        "{} {tier} {rows} rows {bytes} B {span} {}",
        p.path, p.project
    )
}

/// The plan as the catalog stands, no cache started (src/cache T5):
/// the plan is the report.
fn plan(store: &Store, config: &Config, io: &mut Io<'_>) -> Done {
    let plan = Cache::plan_at(store, config, (io.now)());
    let plan = plan.map_err(fail)?;
    say(io, &plan.to_string())?;
    Ok(u8::from(plan.estimate > plan.limit))
}
