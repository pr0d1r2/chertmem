//! Query the pruned partitions in parallel and merge (src/search T2):
//! std threads, each with its own reading store, so 1 connection per
//! partition and never `ATTACH` (root R5). Each partition keeps its best
//! `keep` rows; the order is total (V4) and a row's score ignores its
//! partition, so the merged best equal the best of all rows (V2).

#[cfg(test)]
mod tests;

use std::num::NonZeroUsize;
use std::path::Path;
use std::thread;

use super::Error;
use super::rank::{self, Rater, Scored};
use crate::config::Config;
use crate::store::{Filter, Mode, Observation, Store};

/// What every worker needs: how to open its store, the store's filter,
/// and the query's rater.
pub(super) struct Job<'a> {
    pub(super) cfg: &'a Config,
    pub(super) root: &'a Path,
    pub(super) filter: &'a Filter,
    pub(super) rater: &'a Rater,
}

/// The best `rater.keep` rows across the partitions at `paths`.
pub(super) fn parallel(
    job: &Job<'_>,
    paths: &[String],
) -> Result<Vec<Scored>, Error> {
    let groups = split(paths, workers(paths.len()));
    let found = thread::scope(|s| {
        let jobs: Vec<_> =
            groups.iter().map(|g| s.spawn(|| group(job, g))).collect();
        let joined = jobs.into_iter().map(thread::ScopedJoinHandle::join);
        joined
            .map(|r| r.map_err(|_| Error::Thread).and_then(|done| done))
            .collect::<Result<Vec<_>, _>>()
    })?;
    Ok(rank::top(
        found.into_iter().flatten().collect(),
        job.rater.keep,
    ))
}

/// 1 worker: its own store, the best of each of its partitions.
fn group(job: &Job<'_>, paths: &[String]) -> Result<Vec<Scored>, Error> {
    let mut store = Store::open(job.cfg, job.root, Mode::Read)?;
    let mut best = Vec::new();
    for path in paths {
        best.extend(job.rater.best(candidates(&mut store, path, job)?));
    }
    Ok(best)
}

/// The rows of the partition at `path` that `job` may score.
fn candidates(
    store: &mut Store,
    path: &str,
    job: &Job<'_>,
) -> Result<Vec<Observation>, Error> {
    let rows = store.tail(path, 0, job.filter)?;
    Ok(rows.into_iter().map(|t| t.observation).collect())
}

/// 1 thread per partition, at most 1 per core.
fn workers(parts: usize) -> usize {
    let cores = thread::available_parallelism().map_or(1, NonZeroUsize::get);
    cores.min(parts).max(1)
}

/// `paths` dealt round-robin into `n` groups; none empty when `n` does
/// not exceed the paths.
fn split(paths: &[String], n: usize) -> Vec<Vec<String>> {
    (0..n)
        .map(|i| paths.iter().skip(i).step_by(n).cloned().collect())
        .filter(|g: &Vec<String>| !g.is_empty())
        .collect()
}
