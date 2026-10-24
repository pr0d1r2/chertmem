//! A session's rows in the store's `(ts, id)` order, around 1 ref
//! (src/search I, root I `timeline`).

#[cfg(test)]
mod tests;

use super::{Engine, Error};
use crate::store::{Filter, Lookup, Observation, Ref};

/// 1 timeline row (root I: `{id, kind, ts}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub reference: Ref,
    pub kind: String,
    pub ts: u64,
}

impl Engine {
    /// Up to `limit` (else `search.default_limit`) rows of `session` by
    /// `(ts, id)`: centred on `around` when given, searched in its
    /// project; else the session's first rows in every project.
    ///
    /// # Errors
    /// `around` dropped ([`Error::Dropped`]); a failed store read.
    pub fn timeline(
        &mut self,
        session: &str,
        around: Option<&Ref>,
        limit: Option<u32>,
    ) -> Result<Vec<Entry>, Error> {
        let project = around.map(|r| self.anchor(r)).transpose()?;
        let filter = Filter {
            project,
            ..Filter::default()
        };
        let mut rows = self.store.observations(&filter)?;
        rows.retain(|o| o.session == session);
        let at =
            around.and_then(|r| rows.iter().position(|o| o.reference == *r));
        let picked = window(rows, at.unwrap_or(0), self.limit(limit));
        Ok(picked.into_iter().map(entry).collect())
    }

    /// The project of the row `reference` names.
    fn anchor(&mut self, reference: &Ref) -> Result<String, Error> {
        let found = self.store.get(std::slice::from_ref(reference))?;
        match found.into_iter().next() {
            Some(Lookup::Found(row)) => Ok(row.project),
            _ => Err(Error::Dropped {
                reference: reference.to_string(),
            }),
        }
    }
}

/// `k` rows with `at` in the middle, shifted to stay inside `rows`.
fn window(rows: Vec<Observation>, at: usize, k: usize) -> Vec<Observation> {
    // RED: no window yet.
    let _ = (at, k);
    rows.into_iter().take(0).collect()
}

fn entry(o: Observation) -> Entry {
    Entry {
        reference: o.reference,
        kind: o.kind,
        ts: o.ts,
    }
}
