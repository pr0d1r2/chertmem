//! Every suite (benches C, T3): the root C latency target it measures,
//! the fixtures it runs on, and its setup, or what it waits for.

#[cfg(test)]
mod tests;

use crate::fixture::{Fixture, Res, Spec};
use crate::ops::{hook, read, search, spawn, text};

/// 1 measured call, its setup done.
pub(crate) type Op = Box<dyn FnMut() -> Res<()>>;
/// A suite's setup on a built fixture.
pub(crate) type Make = fn(&Fixture) -> Res<Op>;

/// The fixtures a suite runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Grid {
    /// Every stated size at 52 partitions, every partition count at
    /// 10^5 rows (root V14).
    Store,
    /// Text only: the bodies of the smallest fixture.
    Text,
}

/// A root C latency target, as `SPEC.md` states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Target {
    pub(crate) name: &'static str,
    pub(crate) ms: u64,
}

pub(crate) const HOOK_WRITE: Target = Target {
    name: "hook write",
    ms: 5,
};
pub(crate) const RECENT: Target = Target {
    name: "recent-window search",
    ms: 10,
};
pub(crate) const GET: Target = Target {
    name: "get_observations",
    ms: 2,
};
pub(crate) const RERANK: Target = Target {
    name: "semantic re-rank",
    ms: 20,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Suite {
    /// The criterion group: `<id>/<size>-p<parts>`.
    pub(crate) id: &'static str,
    /// The suite's name in `benches/SPEC.md` C.
    pub(crate) spec: &'static str,
    pub(crate) target: Option<Target>,
    pub(crate) grid: Grid,
    /// Its setup on a built fixture.
    pub(crate) make: Make,
}

/// Every fixture's seed.
pub(crate) const SEED: u64 = 2026;

const fn suite(
    (id, spec): (&'static str, &'static str),
    target: Option<Target>,
    grid: Grid,
    make: Make,
) -> Suite {
    Suite {
        id,
        spec,
        target,
        grid,
        make,
    }
}

pub(crate) const ALL: [Suite; 12] = [
    suite(
        ("hook_write", "hook write"),
        Some(HOOK_WRITE),
        Grid::Store,
        spawn::write,
    ),
    suite(
        ("session_start", "injecting session-start"),
        Some(HOOK_WRITE),
        Grid::Store,
        hook::session_start,
    ),
    suite(
        ("prompt_inject", "opt-in injecting user-prompt-submit"),
        Some(HOOK_WRITE),
        Grid::Store,
        hook::prompt,
    ),
    suite(
        ("get_observations", "get_observations"),
        Some(GET),
        Grid::Store,
        read::get,
    ),
    suite(
        ("recent_warm", "recent-window search"),
        Some(RECENT),
        Grid::Store,
        search::recent_warm,
    ),
    suite(
        ("recent_cold", "recent-window search"),
        Some(RECENT),
        Grid::Store,
        search::recent_cold,
    ),
    suite(
        ("cross_partition", "cross-partition search"),
        None,
        Grid::Store,
        search::cross,
    ),
    suite(
        ("semantic_rerank", "semantic re-rank"),
        Some(RERANK),
        Grid::Store,
        search::semantic,
    ),
    suite(
        ("cache_warmup", "cache warm-up"),
        None,
        Grid::Store,
        read::warm_up,
    ),
    suite(
        ("partition_scan", "partition scan"),
        None,
        Grid::Store,
        read::scan,
    ),
    suite(
        ("compress", "compress & expand"),
        None,
        Grid::Text,
        text::compress,
    ),
    suite(
        ("expand", "compress & expand"),
        None,
        Grid::Text,
        text::expand,
    ),
];

/// (size, partitions) of the store grid: the stated sizes at a year of
/// weeks, then the stated partition counts at 10^5 rows.
const STORE: [(u64, u64); 7] = [
    (1000, 52),
    (10_000, 52),
    (100_000, 52),
    (1_000_000, 52),
    (100_000, 1),
    (100_000, 4),
    (100_000, 520),
];

/// The fixtures `suite` runs on.
pub(crate) fn grid(suite: &Suite) -> Vec<Spec> {
    let at = |(size, parts)| Spec::new(SEED, size, parts);
    match suite.grid {
        Grid::Store => STORE.into_iter().map(at).collect(),
        Grid::Text => vec![at((1000, 1))],
    }
}
