//! Searches through `search::Engine`: the recent window from a warm
//! cache and from disk, every partition, and the semantic re-rank.

#[cfg(test)]
mod tests;

use std::hint::black_box;

use chertmem::cache::Handle;
use chertmem::config::{Config, Provider};
use chertmem::search::{Engine, Query};

use super::warm;
use crate::fixture::{DAY, END, Fixture, Res};
use crate::suites::Op;
use crate::words::word;

/// `cache.window`'s default: 28 days back from the clock.
const WINDOW: u64 = 28 * DAY;

/// 2 common words, as an agent's search has them: ranks 27 and 43.
pub(crate) fn text() -> String {
    format!("{} {}", word(27), word(43))
}

/// The query: recent or over all time, semantic or not.
fn query(engine: &Engine, recent: bool, semantic: bool) -> Query {
    Query {
        text: text(),
        since: recent.then(|| END.saturating_sub(WINDOW)),
        project: Some(engine.project().to_owned()),
        semantic,
        ..Query::default()
    }
}

/// An engine on `cfg` running `q`, from `cache` when given; it must
/// find something once before it is measured.
fn searching(
    fx: &Fixture,
    cfg: &Config,
    q: Q,
    cache: Option<Handle>,
) -> Res<Op> {
    let mut engine = Engine::open(cfg, &fx.root())?;
    let q = query(&engine, q.recent, q.semantic);
    if engine.run(&q, cache.as_ref())?.is_empty() {
        return Err(format!("{:?} found nothing", q.text).into());
    }
    Ok(Box::new(move || {
        black_box(engine.run(&q, cache.as_ref())?);
        Ok(())
    }))
}

/// What a search suite asks.
#[derive(Debug, Clone, Copy)]
struct Q {
    recent: bool,
    semantic: bool,
}

const RECENT: Q = Q {
    recent: true,
    semantic: false,
};

/// Recent-window search answered from a warm cache.
pub(crate) fn recent_warm(fx: &Fixture) -> Res<Op> {
    let cfg = fx.config()?;
    let cache = warm(fx, &cfg)?;
    searching(fx, &cfg, RECENT, Some(cache))
}

/// Recent-window search with no cache: the pruned partitions on disk.
pub(crate) fn recent_cold(fx: &Fixture) -> Res<Op> {
    searching(fx, &fx.config()?, RECENT, None)
}

/// Cross-partition search: all time, every partition on disk.
pub(crate) fn cross(fx: &Fixture) -> Res<Op> {
    let all = Q {
        recent: false,
        semantic: false,
    };
    searching(fx, &fx.config()?, all, None)
}

/// Recent-window search re-ranked by embeddings from the stand-in
/// ollama (`crate::ollama`), keyword rows from a warm cache.
pub(crate) fn semantic(fx: &Fixture) -> Res<Op> {
    let mut cfg = fx.config()?;
    cfg.embedding.provider = Provider::Ollama;
    let cache = warm(fx, &cfg)?;
    let q = Q {
        recent: true,
        semantic: true,
    };
    searching(fx, &cfg, q, Some(cache))
}
