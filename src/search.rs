//! Targeted retrieval (src/search G): prune partitions, query each in
//! parallel, merge top-k under global term stats; 1 scoring function
//! for cached and disk rows (D17), an optional vector re-rank (D18).

#[cfg(test)]
mod tests;

mod blend;
mod error;
mod merge;
mod ollama;
mod postings;
mod rank;
mod snippet;
mod terms;
mod timeline;
mod token;

use std::any::Any;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use error::Error;
pub use timeline::Entry;

use crate::cache::{Handle, Index, Log, Rows};
use crate::config::{Config, Grammar, Provider};
use crate::grammar;
use crate::store::{self, Filter, Mode, Ref, Store};
use ollama::Ollama;
use postings::Postings;
use rank::{Rater, Scored};
use terms::Terms;

/// Keyword best per wanted hit that a semantic query re-ranks.
const POOL: usize = 4;

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
    /// The disk path's worker stores, kept across queries (T6).
    pool: merge::Pool,
    /// `OLLAMA_HOST`, read only when `embedding.provider = "ollama"`.
    host: String,
    warn: Warn,
}

/// Where an engine warns, at most once (T8): stderr, as the cache's
/// default log, which `chert mcp` keeps apart from its protocol.
struct Warn {
    log: Log,
    said: bool,
}

impl Warn {
    fn stderr() -> Self {
        Self {
            log: Arc::new(|text: &str| eprintln!("{text}")),
            said: false,
        }
    }

    /// Log `text` unless this engine warned before.
    fn once(&mut self, text: &str) {
        if !self.said {
            self.said = true;
            (self.log)(text);
        }
    }
}

impl fmt::Debug for Warn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Warn")
            .field("said", &self.said)
            .finish_non_exhaustive()
    }
}

impl Engine {
    /// Open the data dir `cfg` names for the project at `project_root`.
    ///
    /// # Errors
    /// No data dir, an unusable store or term file.
    pub fn open(cfg: &Config, project_root: &Path) -> Result<Self, Error> {
        let host = match cfg.embedding.provider {
            Provider::Ollama => {
                std::env::var("OLLAMA_HOST").unwrap_or_default()
            }
            Provider::Off | Provider::Openai => String::new(),
        };
        Self::with_host(cfg, project_root, host)
    }

    fn with_host(
        cfg: &Config,
        project_root: &Path,
        host: String,
    ) -> Result<Self, Error> {
        let mut store = Store::open(cfg, project_root, Mode::Read)?;
        store.keep_open(merge::OWN);
        let data = cfg.storage.data_dir.as_deref();
        let terms = Terms::open(data.ok_or(store::Error::NoDataDir)?)?;
        Ok(Self {
            cfg: cfg.clone(),
            root: project_root.to_path_buf(),
            store,
            terms,
            pool: merge::Pool::default(),
            host,
            warn: Warn::stderr(),
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
    /// A failed store, cache, term file or embedding call.
    pub fn run(
        &mut self,
        q: &Query,
        cache: Option<&Handle>,
    ) -> Result<Vec<Hit>, Error> {
        let embedder = self.embedder(q.semantic)?;
        let asked = Asked::new(&q.text, self.cfg.compression.grammar);
        let Some(rater) = self.rater(q, asked.terms, embedder.is_some())?
        else {
            return Ok(Vec::new());
        };
        let best = self.candidates(&filter(q), cache, &rater)?;
        let alpha = self.cfg.search.alpha;
        let ranked = match embedder {
            Some(e) => blend::rerank(&e, &asked.text, best, alpha)?,
            None => best,
        };
        let hits = ranked.into_iter().take(self.limit(q.limit));
        Ok(hits.map(|s| hit(s, &rater.terms)).collect())
    }

    /// The query's rater for `terms` under fresh global stats; `None`
    /// when there is no term.
    fn rater(
        &mut self,
        q: &Query,
        terms: Vec<String>,
        rerank: bool,
    ) -> Result<Option<Rater>, Error> {
        if terms.is_empty() {
            return Ok(None);
        }
        self.terms.refresh(&mut self.store)?;
        let limit = self.limit(q.limit);
        Ok(Some(Rater {
            stats: self.terms.stats(&terms)?,
            terms,
            kind: q.kind.clone(),
            session: q.session.clone(),
            keep: limit.saturating_mul(if rerank { POOL } else { 1 }),
        }))
    }

    /// The keyword best: the cache's rows, or every pruned partition.
    fn candidates(
        &mut self,
        filter: &Filter,
        cache: Option<&Handle>,
        rater: &Rater,
    ) -> Result<Vec<Scored>, Error> {
        if let Some(handle) = cache {
            return cached(handle, filter, rater);
        }
        let parts = self.store.partitions(filter)?;
        let paths: Vec<String> = parts.into_iter().map(|p| p.path).collect();
        let job = merge::Job {
            cfg: &self.cfg,
            root: &self.root,
            generation: self.store.generation()?,
            filter,
            rater,
        };
        merge::parallel(&job, &mut self.pool, &paths)
    }

    /// The provider a query may call; `None` = no network (V3).
    /// A provider without a v1 client ranks by keyword and warns once
    /// (T8): `src/mcp` asks every search to be semantic.
    fn embedder(&mut self, semantic: bool) -> Result<Option<Ollama>, Error> {
        if !semantic {
            return Ok(None);
        }
        match self.cfg.embedding.provider {
            Provider::Off => Ok(None),
            Provider::Ollama => Ollama::new(&self.host).map(Some),
            Provider::Openai => {
                let no = Error::Unsupported { provider: "openai" };
                self.warn.once(&no.to_string());
                Ok(None)
            }
        }
    }

    /// `limit`, else `search.default_limit`.
    fn limit(&self, limit: Option<u32>) -> usize {
        let n = limit.unwrap_or(self.cfg.search.default_limit);
        usize::try_from(n).unwrap_or(usize::MAX)
    }
}

/// What a query text asks, as scored and as embedded (src/search R3).
struct Asked {
    /// The text the re-rank embeds: the compressed query, like the
    /// stored bodies it is compared with; the raw text when that holds
    /// no term.
    text: String,
    /// The compressed text's terms and the plain form of every word
    /// the grammar keeps on its own, distinct and sorted: a word kept
    /// long inside a path or code is found too (T7), and a term alike
    /// in both forms counts once.
    terms: Vec<String>,
}

impl Asked {
    fn new(text: &str, grammar: Grammar) -> Self {
        let intensity = grammar.intensity();
        let compressed = grammar::compress(text, intensity);
        let mut terms = token::terms(&compressed);
        let shown = if terms.is_empty() { text } else { &compressed };
        let shown = shown.to_owned();
        let kept = token::tokens(text).into_iter().filter(|t| {
            !token::tokens(&grammar::compress(t, intensity)).is_empty()
        });
        terms.extend(kept);
        terms.sort_unstable();
        terms.dedup();
        Self { text: shown, terms }
    }
}

/// The keyword index a cache searched by [`Engine::run`] keeps (T9):
/// start it with [`crate::cache::Cache::start_indexed`] so a warm
/// search reads only the rows its terms are in.
#[must_use]
pub fn index() -> Box<dyn Index> {
    Box::new(Postings::default())
}

/// The keyword best through `handle`: the warm window's postings (T9),
/// else the rows [`Handle::query`] reads from the store.
fn cached(
    handle: &Handle,
    filter: &Filter,
    rater: &Rater,
) -> Result<Vec<Scored>, Error> {
    let warm = handle.with_index(filter, |rows, i| lent(rater, rows, i))?;
    match warm {
        Some(best) => Ok(best),
        None => Ok(rater.best(handle.query(filter)?)),
    }
}

/// The best of the window's lent `rows`: those [`index`]'s postings
/// hold a term in, scored from their counts; every row scored whole
/// when the cache keeps another index.
fn lent(rater: &Rater, rows: Rows<'_>, index: &dyn Index) -> Vec<Scored> {
    let any: &dyn Any = index;
    let hits = any
        .downcast_ref::<Postings>()
        .and_then(|p| p.hits(&rater.terms));
    let Some(hits) = hits else {
        return rater.best(rows.to_vec());
    };
    let found = hits
        .into_iter()
        .filter_map(|(key, c)| Some((rows.get(key)?, c)));
    rater.best_counted(found)
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
