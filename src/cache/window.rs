//! The window's rows in memory, keyed by `(ts, id)` as the store orders
//! them, and per partition how far they were read: its seq and its
//! `PRAGMA data_version` (src/cache C, V2). Every row added or replaced
//! reaches the index too (T6).

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use super::index::{Index, Key};
use crate::store::{Error, Filter, Observation, Store};

/// How far 1 partition was read.
#[derive(Debug, Default)]
struct Part {
    /// `PRAGMA data_version` at the last tail; `None` while no file.
    version: Option<u64>,
    /// The highest seq kept: the next tail starts after it, never at a
    /// ULID (V2).
    seq: i64,
}

/// The rows of 1 project from `since` on.
#[derive(Debug)]
pub(super) struct Window {
    /// The project and `since`: what every tail asks for.
    filter: Filter,
    since: u64,
    /// The catalog's `data_version` when its partitions were listed.
    catalog: u64,
    /// The catalog's `generation` at the load (D15).
    generation: u64,
    parts: BTreeMap<String, Part>,
    rows: BTreeMap<Key, Observation>,
}

/// Whether a refreshed window may answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Fresh {
    Current,
    /// A delete, redaction or rewrite bumped `generation` since the
    /// load: a tail cannot see it, so the window must be loaded again
    /// (V3).
    Stale,
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
            catalog: store.catalog_version()?,
            generation: store.generation()?,
            parts: BTreeMap::new(),
            rows: BTreeMap::new(),
        };
        window.list(store)?;
        Ok(window)
    }

    /// Add every partition the catalog names for the window.
    fn list(&mut self, store: &Store) -> Result<(), Error> {
        for p in store.partitions(&self.filter)? {
            self.parts.entry(p.path).or_default();
        }
        Ok(())
    }

    /// The partition paths, by path.
    pub(super) fn paths(&self) -> Vec<String> {
        self.parts.keys().cloned().collect()
    }

    /// Catch up before a query (D15): a moved catalog is checked for a
    /// new `generation`, then listed for new partitions; every
    /// partition is synced, `index` fed as the rows are.
    pub(super) fn refresh(
        &mut self,
        store: &mut Store,
        index: &mut dyn Index,
    ) -> Result<Fresh, Error> {
        let catalog = store.catalog_version()?;
        if catalog != self.catalog {
            if store.generation()? != self.generation {
                return Ok(Fresh::Stale);
            }
            self.catalog = catalog;
            self.list(store)?;
        }
        for path in self.paths() {
            self.sync(store, index, &path)?;
        }
        Ok(Fresh::Current)
    }

    /// Tail the partition at `path` past its last seq, when its
    /// `data_version` moved since the last tail; `index` sees each row
    /// with the one it replaces at its key.
    pub(super) fn sync(
        &mut self,
        store: &mut Store,
        index: &mut dyn Index,
        path: &str,
    ) -> Result<(), Error> {
        let version = store.data_version(path)?;
        let part = self.parts.entry(path.to_owned()).or_default();
        if version == part.version {
            return Ok(());
        }
        part.version = version;
        for t in store.tail(path, part.seq, &self.filter)? {
            part.seq = part.seq.max(t.seq);
            let o = t.observation;
            let key = (o.ts, o.id.clone());
            let _ = &index;
            self.rows.insert(key, o);
        }
        Ok(())
    }

    /// The rows `q` wants, lent, when the window holds all of them:
    /// the same project and `since` inside the window; else `None`.
    pub(super) fn view(&self, q: &Filter) -> Option<Rows<'_>> {
        let since = q.since.filter(|&s| s >= self.since)?;
        if q.project != self.filter.project {
            return None;
        }
        Some(Rows {
            map: &self.rows,
            since,
            until: q.until,
        })
    }
}

/// The window's rows a filter wants, lent under the cache lock
/// ([`super::Handle::with_index`]): its `[since, until)` of the window.
#[derive(Debug, Clone, Copy)]
pub struct Rows<'a> {
    map: &'a BTreeMap<Key, Observation>,
    since: u64,
    until: Option<u64>,
}

impl<'a> Rows<'a> {
    /// The row at `key` when the filter wants it: an index posting a
    /// row outside the filter's ts range gets `None`.
    #[must_use]
    pub fn get(&self, key: &Key) -> Option<&'a Observation> {
        let (ts, _) = key;
        let inside = *ts >= self.since && self.until.is_none_or(|u| *ts < u);
        inside.then(|| self.map.get(key)).flatten()
    }

    /// Every row the filter wants, in the store's `(ts, id)` order.
    pub fn range(&self) -> impl Iterator<Item = &'a Observation> + 'a {
        let until = self.until;
        let rows = self.map.range((self.since, String::new())..);
        let wanted =
            rows.take_while(move |((ts, _), _)| until.is_none_or(|u| *ts < u));
        wanted.map(|(_, o)| o)
    }

    /// The rows as [`super::Handle::query`] answers them, cloned.
    #[must_use]
    pub fn to_vec(&self) -> Vec<Observation> {
        self.range().cloned().collect()
    }
}
