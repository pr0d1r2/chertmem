//! `chert mine`: every partition of this project mined, a sealed one
//! once, its findings kept in the finding store; a hot one mined on
//! demand and never kept (src/mine C).

#[cfg(test)]
mod tests;

use super::ledger::{Key, Ledger};
use super::{Error, Finding};
use crate::config::Config;
use crate::store::{self, Filter, Store};

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
    // RED stub: reads the catalog, keeps nothing worth keeping.
    let dir = config.storage.data_dir.as_deref();
    let mut ledger = Ledger::open(dir.ok_or(store::Error::NoDataDir)?)?;
    let min = config.mining.min_occurrences;
    let filter = Filter {
        since,
        ..Filter::default()
    };
    for p in store.partitions(&filter)? {
        let key = Key::of(&p, store.project(), min);
        if ledger.get(&key)?.is_none() {
            ledger.put(&key, &[])?;
        }
    }
    Ok(Vec::new())
}
