//! Targeted retrieval (src/search G): prune partitions, query each in
//! parallel, merge top-k under global term stats; 1 scoring function
//! for cached and disk rows (D17), an optional vector re-rank (D18).

#[cfg(test)]
mod tests;

mod error;
mod merge;
mod rank;
mod snippet;
mod terms;
mod timeline;
mod token;

use std::path::{Path, PathBuf};

pub use error::Error;
pub use timeline::Entry;

use crate::cache::Handle;
use crate::config::Config;
use crate::store::{self, Filter, Mode, Ref, Store};
use rank::{Rater, Scored};
use terms::Terms;

/// 1 search (src/search I); `None` = unbounded, `limit` defaulting to
/// `search.default_limit`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    pub text: String,
    pub limit: Option<u32>,
    /// Milliseconds since the Unix epoch.
    pub since: Option<u64>,
    /// A project id ([`store::project_id`]); `None` = every project.
    pub project: Option<String>,
    pub kind: Option<String>,
    pub session: Option<String>,
    /// Re-rank by embeddings, when `embedding.provider` opts in (V3).
    pub semantic: bool,
}

/// 1 ranked row (src/search I, root I `search`).
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub reference: Ref,
    pub score: f64,
    pub snippet: String,
    pub session: String,
    pub ts: u64,
}

/// A search over 1 data dir: its reading store and its term stats.
#[derive(Debug)]
pub struct Engine {
    cfg: Config,
    root: PathBuf,
    store: Store,
    terms: Terms,
}

impl Engine {
    /// Open the data dir `cfg` names for the project at `project_root`.
    ///
    /// # Errors
    /// No data dir, an unusable store or term file.
    pub fn open(cfg: &Config, project_root: &Path) -> Result<Self, Error> {
        let store = Store::open(cfg, project_root, Mode::Read)?;
        let data = cfg.storage.data_dir.as_deref();
        let terms = Terms::open(data.ok_or(store::Error::NoDataDir)?)?;
        Ok(Self {
            cfg: cfg.clone(),
            root: project_root.to_path_buf(),
            store,
            terms,
        })
    }

    /// The project id of the root this engine opened.
    #[must_use]
    pub fn project(&self) -> &str {
        self.store.project()
    }

    /// The best hits for `q`, best first (V4): from `cache` when given
    /// (it answers from memory or the store, V1), else from every
    /// pruned partition in parallel (T2). Scores ignore where a row was
    /// read (V1, V2).
    ///
    /// # Errors
    /// A failed store, cache or term file.
    pub fn run(
        &mut self,
        q: &Query,
        cache: Option<&Handle>,
    ) -> Result<Vec<Hit>, Error> {
        let Some(rater) = self.rater(q)? else {
            return Ok(Vec::new());
        };
        let best = self.candidates(&filter(q), cache, &rater)?;
        let hits = best.into_iter().take(self.limit(q.limit));
        Ok(hits.map(|s| hit(s, &rater.terms)).collect())
    }

    /// The query's rater under fresh global stats; `None` when the text
    /// holds no term.
    fn rater(&mut self, q: &Query) -> Result<Option<Rater>, Error> {
        let terms = token::terms(&q.text);
        if terms.is_empty() {
            return Ok(None);
        }
        self.terms.refresh(&mut self.store)?;
        Ok(Some(Rater {
            stats: self.terms.stats(&terms)?,
            terms,
            kind: q.kind.clone(),
            session: q.session.clone(),
            keep: self.limit(q.limit),
        }))
    }

    /// The keyword best of every pruned partition; the cache's rows
    /// land in T3.
    fn candidates(
        &mut self,
        filter: &Filter,
        cache: Option<&Handle>,
        rater: &Rater,
    ) -> Result<Vec<Scored>, Error> {
        if cache.is_some() {
            return Ok(Vec::new());
        }
        let parts = self.store.partitions(filter)?;
        let paths: Vec<String> = parts.into_iter().map(|p| p.path).collect();
        let job = merge::Job {
            cfg: &self.cfg,
            root: &self.root,
            filter,
            rater,
        };
        merge::parallel(&job, &paths)
    }

    /// `limit`, else `search.default_limit`.
    fn limit(&self, limit: Option<u32>) -> usize {
        let n = limit.unwrap_or(self.cfg.search.default_limit);
        usize::try_from(n).unwrap_or(usize::MAX)
    }
}

/// The store's side of `q`.
fn filter(q: &Query) -> Filter {
    Filter {
        project: q.project.clone(),
        since: q.since,
        until: None,
    }
}

fn hit(s: Scored, terms: &[String]) -> Hit {
    Hit {
        snippet: snippet::around(&s.row.body, terms),
        reference: s.row.reference,
        score: s.score,
        session: s.row.session,
        ts: s.row.ts,
    }
}
