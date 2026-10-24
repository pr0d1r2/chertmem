//! The window's rows in memory, keyed by `(ts, id)` as the store orders
//! them, and the partitions they come from (src/cache C).

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use crate::store::{Error, Filter, Observation, Store};

/// The rows of 1 project from `since` on.
#[derive(Debug)]
pub(super) struct Window {
    /// The project and `since`: what every tail asks for.
    filter: Filter,
    since: u64,
    parts: BTreeSet<String>,
    rows: BTreeMap<(u64, String), Observation>,
}

impl Window {
    /// An empty window of `store`'s project from `since`, its
    /// partitions listed from the catalog.
    pub(super) fn begin(store: &Store, since: u64) -> Result<Self, Error> {
        let filter = Filter {
            project: Some(store.project().to_owned()),
            since: Some(since),
            until: None,
        };
        let mut window = Self {
            filter,
            since,
            parts: BTreeSet::new(),
            rows: BTreeMap::new(),
        };
        window.list(store)?;
        Ok(window)
    }

    /// Add every partition the catalog names for the window.
    fn list(&mut self, store: &Store) -> Result<(), Error> {
        for p in store.partitions(&self.filter)? {
            self.parts.insert(p.path);
        }
        Ok(())
    }

    /// The partition paths, by path.
    pub(super) fn paths(&self) -> Vec<String> {
        self.parts.iter().cloned().collect()
    }

    /// Load the partition at `path`, in seq order (src/store tail).
    pub(super) fn sync(
        &mut self,
        store: &mut Store,
        path: &str,
    ) -> Result<(), Error> {
        for t in store.tail(path, 0, &self.filter)? {
            let o = t.observation;
            self.rows.insert((o.ts, o.id.clone()), o);
        }
        Ok(())
    }

    /// The rows `q` wants when the window holds all of them: the same
    /// project and `since` inside the window; else `None`.
    pub(super) fn answer(&self, q: &Filter) -> Option<Vec<Observation>> {
        let since = q.since.filter(|&s| s >= self.since)?;
        if q.project != self.filter.project {
            return None;
        }
        let rows = self.rows.range((since, String::new())..);
        let wanted =
            rows.take_while(|((ts, _), _)| q.until.is_none_or(|u| *ts < u));
        Some(wanted.map(|(_, o)| o.clone()).collect())
    }
}
