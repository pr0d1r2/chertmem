//! `chert partitions [list|seal|compact|drop|repartition]` and `chert
//! cache [plan]` (root I): the store's partitions and the cache's fit.

#[cfg(test)]
mod tests;

use std::sync::Arc;

use clap::Subcommand;

use super::{Done, Io, fail, load, open, say};
use crate::cache::{Cache, Env};
use crate::config::Config;
use crate::store::{Filter, Partition, Store};

#[derive(Debug, Subcommand)]
pub(super) enum Verb {
    /// The store's partitions; `list` when no action is given
    Partitions {
        #[command(subcommand)]
        action: Option<Partitions>,
    },
    /// The recent-window cache; `plan` when no action is given
    Cache {
        #[command(subcommand)]
        action: Option<CachePlan>,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum Partitions {
    /// Every partition: path, tier, rows, bytes, time span, project
    List,
    /// Seal a partition: warm, refusing appends
    Seal { path: String },
    /// Seal, then zstd a partition into the cold tier
    Compact { path: String },
    /// Drop a partition; its refs answer `dropped`
    Drop { path: String },
    /// Rewrite partitions written under another `partition.*` spec
    Repartition,
}

#[derive(Debug, Subcommand)]
pub(super) enum CachePlan {
    /// The window's estimate against `cache.max_bytes`; exit 1 when over
    Plan,
}

pub(super) fn run(verb: Verb, io: &mut Io<'_>) -> Done {
    let config = load(io)?;
    let store = open(&config, io)?;
    match verb {
        Verb::Partitions { action } => {
            partitions(store, &action.unwrap_or(Partitions::List), io)
        }
        Verb::Cache { action: _ } => plan(store, &config, io),
    }
}

fn partitions(mut store: Store, action: &Partitions, io: &mut Io<'_>) -> Done {
    let done = match action {
        Partitions::List => return list(&store, io),
        Partitions::Seal { path } => store.seal(path),
        Partitions::Compact { path } => store.compact(path),
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

/// The plan as the catalog stands; the cache started only to plan, its
/// log silent: the plan is the report.
fn plan(store: Store, config: &Config, io: &mut Io<'_>) -> Done {
    let env = Env {
        now: io.now,
        memory: || None,
        log: Arc::new(|_: &str| {}),
    };
    let plan = Cache::start_with(store, config, env).plan();
    let plan = plan.map_err(fail)?;
    say(io, &plan.to_string())?;
    Ok(u8::from(plan.estimate > plan.limit))
}
