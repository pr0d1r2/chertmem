//! Reads: `get_observations`' lookup by ref, a partition scan and a
//! cache warm-up, each on the fixture as cached.

#[cfg(test)]
mod tests;

use std::hint::black_box;

use chertmem::store::{Filter, Lookup, Ref, Store};

use super::{start, wait};
use crate::fixture::{END, Fixture, Res, WEEK};
use crate::suites::Op;

/// Refs 1 `get_observations` asks for, as an agent does after a search.
const REFS: usize = 5;

/// The newest partition, whole: every bucket is 1 week.
pub(crate) fn newest(store: &Store) -> Filter {
    Filter {
        project: Some(store.project().to_owned()),
        since: Some(END.saturating_sub(WEEK)),
        until: None,
    }
}

/// [`REFS`] refs spread over the newest partition.
pub(crate) fn sample(store: &mut Store) -> Res<Vec<Ref>> {
    let rows = store.observations(&newest(store))?;
    let step = rows.len().checked_div(REFS).unwrap_or(1).max(1);
    let refs = rows.into_iter().step_by(step).take(REFS);
    Ok(refs.map(|o| o.reference).collect())
}

/// `get_observations`: [`REFS`] refs resolved to their rows.
pub(crate) fn get(fx: &Fixture) -> Res<Op> {
    let mut store = fx.store()?;
    let refs = sample(&mut store)?;
    let found = store.get(&refs)?;
    if !found.iter().all(|l| matches!(l, Lookup::Found(_))) {
        return Err("a sampled ref did not resolve".into());
    }
    Ok(Box::new(move || {
        black_box(store.get(&refs)?);
        Ok(())
    }))
}

/// A partition scan: every row of the newest partition.
pub(crate) fn scan(fx: &Fixture) -> Res<Op> {
    let mut store = fx.store()?;
    let filter = newest(&store);
    if store.observations(&filter)?.is_empty() {
        return Err("the newest partition is empty".into());
    }
    Ok(Box::new(move || {
        black_box(store.observations(&filter)?);
        Ok(())
    }))
}

/// A cache warm-up, as `chert mcp` starts: open, load the window.
pub(crate) fn warm_up(fx: &Fixture) -> Res<Op> {
    let (fx, cfg) = (fx.clone(), fx.config()?);
    Ok(Box::new(move || {
        let cache = start(&fx, &cfg)?;
        wait(&cache)
    }))
}
