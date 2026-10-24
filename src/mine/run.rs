//! `chert mine`: every partition of this project mined, a sealed one
//! once, its findings kept in the finding store; a hot one mined on
//! demand and never kept (src/mine C).

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use super::ledger::{Key, Ledger};
use super::{Error, Finding, signal};
use crate::config::Config;
use crate::store::{self, Filter, Observation, Partition, State, Store};

/// The findings of this project's partitions overlapping `[since, ..)`:
/// each partition's, merged by id across partitions, by count (most
/// first), then kind, then id (V1). Every finding carries at least
/// `mining.min_occurrences` refs of rows the store still holds (V2).
/// `mining.enabled = false` finds nothing and writes nothing.
///
/// # Errors
/// A store read fails or the finding store is unusable.
pub fn mine(
    store: &mut Store,
    config: &Config,
    since: Option<u64>,
) -> Result<Vec<Finding>, Error> {
    if !config.mining.enabled {
        return Ok(Vec::new());
    }
    let dir = config.storage.data_dir.as_deref();
    let mut ledger = Ledger::open(dir.ok_or(store::Error::NoDataDir)?)?;
    let (min, filter) = (config.mining.min_occurrences, own(store, since));
    let mut all = Vec::new();
    for p in store.partitions(&filter)? {
        all.extend(partition(store, &mut ledger, &p, min)?);
    }
    Ok(merge(all))
}

/// This project's rows from `since` on.
fn own(store: &Store, since: Option<u64>) -> Filter {
    Filter {
        project: Some(store.project().to_owned()),
        since,
        until: None,
    }
}

/// The findings of `p`: a hot one mined now, a sealed one read from the
/// ledger, mined and kept there when it holds none fresh.
fn partition(
    store: &mut Store,
    ledger: &mut Ledger,
    p: &Partition,
    min: u32,
) -> Result<Vec<Finding>, Error> {
    if p.state == State::Hot {
        return Ok(signal::findings(&rows(store, p)?, min));
    }
    let key = Key::of(p, store.project(), min);
    if let Some(kept) = ledger.get(&key)? {
        return Ok(kept);
    }
    let found = signal::findings(&rows(store, p)?, min);
    ledger.put(&key, &found)?;
    Ok(found)
}

/// Every row of this project in `p`.
fn rows(store: &mut Store, p: &Partition) -> Result<Vec<Observation>, Error> {
    let filter = own(store, None);
    let tailed = store.tail(&p.path, 0, &filter)?;
    Ok(tailed.into_iter().map(|t| t.observation).collect())
}

/// `found` with 1 finding per id, its refs those of every partition in
/// partition order, which is time order (src/store V4).
fn merge(found: Vec<Finding>) -> Vec<Finding> {
    let mut by_id: BTreeMap<String, Finding> = BTreeMap::new();
    for f in found {
        if let Entry::Occupied(mut slot) = by_id.entry(f.id.clone()) {
            let kept = slot.get_mut();
            kept.refs.extend(f.refs);
            kept.count = kept.refs.len();
        } else {
            by_id.insert(f.id.clone(), f);
        }
    }
    let mut merged: Vec<Finding> = by_id.into_values().collect();
    signal::sort(&mut merged);
    merged
}
