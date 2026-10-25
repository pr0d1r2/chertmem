//! `chert partitions [list|seal|compact|drop|repartition]` and `chert
//! cache [plan]` (root I): the store's partitions and the cache's fit.

#[cfg(test)]
mod tests;

use clap::Subcommand;

use super::{Done, Io};

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

pub(super) fn run(verb: Verb, _io: &mut Io<'_>) -> Done {
    super::not_built(verb)
}
