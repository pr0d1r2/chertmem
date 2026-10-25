//! Query the pruned partitions in parallel and merge (src/search T2):
//! std threads, each with its own reading store, so 1 connection per
//! partition and never `ATTACH` (root R5). Each partition gives only the
//! rows its FTS5 index matches (T6) and keeps its best `keep`; the order
//! is total (V4) and a row's score ignores its partition, so the merged
//! best equal the best of all rows (V2).

#[cfg(test)]
mod tests;

use std::num::NonZeroUsize;
use std::path::Path;
use std::thread;

use super::Error;
use super::rank::{self, Rater, Scored};
use crate::config::Config;
use crate::store::{Filter, Mode, Observation, Store};

/// What every worker needs: how to open its store, the store's
/// generation now, the store's filter, and the query's rater.
pub(super) struct Job<'a> {
    pub(super) cfg: &'a Config,
    pub(super) root: &'a Path,
    pub(super) generation: u64,
    pub(super) filter: &'a Filter,
    pub(super) rater: &'a Rater,
}

/// The workers' reading stores, kept from query to query so a query
/// opens no catalog and no partition file again (T6); dropped when the
/// store's generation moves, as a seal, compaction, drop or repartition
/// moves it, so no worker reads a file the catalog let go.
#[derive(Debug, Default)]
pub(super) struct Pool {
    generation: u64,
    stores: Vec<Store>,
}

impl Pool {
    /// At least `n` stores, every one opened in `job`'s generation.
    fn fill(&mut self, job: &Job<'_>, n: usize) -> Result<(), Error> {
        if self.generation != job.generation {
            self.stores.clear();
            self.generation = job.generation;
        }
        while self.stores.len() < n {
            self.stores
                .push(Store::open(job.cfg, job.root, Mode::Read)?);
        }
        Ok(())
    }
}

/// The best `rater.keep` rows across the partitions at `paths`.
pub(super) fn parallel(
    job: &Job<'_>,
    pool: &mut Pool,
    paths: &[String],
) -> Result<Vec<Scored>, Error> {
    let groups = split(paths, workers(paths.len()));
    pool.fill(job, groups.len())?;
    let found = thread::scope(|s| {
        let jobs: Vec<_> = (groups.iter().zip(pool.stores.iter_mut()))
            .map(|(g, store)| s.spawn(move || group(job, store, g)))
            .collect();
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

/// 1 worker on its own store: the best of each of its partitions.
fn group(
    job: &Job<'_>,
    store: &mut Store,
    paths: &[String],
) -> Result<Vec<Scored>, Error> {
    let mut best = Vec::new();
    for path in paths {
        best.extend(job.rater.best(candidates(store, path, job)?));
    }
    Ok(best)
}

/// The rows of the partition at `path` that `job` may score: those its
/// FTS5 index matches for any query term, never every row (T6). The
/// index's `unicode61` tokens equal [`super::token`]'s on ASCII, so
/// every row a term scores in is among them (src/search R6).
fn candidates(
    store: &mut Store,
    path: &str,
    job: &Job<'_>,
) -> Result<Vec<Observation>, Error> {
    let rows = store.matches(path, &job.rater.terms, job.filter)?;
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
