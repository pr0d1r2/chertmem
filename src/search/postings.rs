//! The search's own index on the cache window (src/search T9): per
//! term the rows holding it and how often, per row its token count, so
//! a warm query reads the rows its terms are in, never every row. The
//! counts come from [`super::token`], as [`super::rank::score`]'s do,
//! so a row scores the same bits through either (V1).

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::mem::size_of;

use super::rank::Counts;
use super::token;
use crate::cache::{Index, Key};
use crate::store::Observation;

/// 1 row holding a term: its slot, and the term's count in its body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Posting {
    slot: u32,
    tf: u32,
}

/// Postings over the window's rows, fed by the cache (src/cache R6).
#[derive(Debug, Default)]
pub(crate) struct Postings {
    /// Every term and the rows holding it, each row once.
    terms: HashMap<String, Vec<Posting>>,
    /// Per slot, the row's key; a replaced row's slot stays, unposted.
    keys: Vec<Key>,
    /// Per slot, the tokens in the row's body (BM25's dl).
    lengths: Vec<u32>,
    /// More rows than slots: the index answers nothing, rows are
    /// scored whole.
    full: bool,
}

impl Postings {
    /// Per row holding any of `terms`: its key and counts, `tf` in
    /// `terms` order; `None` when this index cannot answer.
    pub(super) fn hits(&self, terms: &[String]) -> Option<Vec<(&Key, Counts)>> {
        if self.full {
            return None;
        }
        let found = self.gather(terms).into_iter();
        Some(found.filter_map(|(s, tf)| self.counts(s, tf)).collect())
    }

    /// Per slot posted for any of `terms`, each term's tf in order.
    fn gather(&self, terms: &[String]) -> HashMap<u32, Vec<u32>> {
        let mut found: HashMap<u32, Vec<u32>> = HashMap::new();
        let posted = terms
            .iter()
            .enumerate()
            .filter_map(|(i, t)| self.terms.get(t).map(|list| (i, list)));
        for (i, p) in
            posted.flat_map(|(i, list)| list.iter().map(move |p| (i, p)))
        {
            let tf = found.entry(p.slot).or_insert_with(|| zeros(terms));
            if let Some(n) = tf.get_mut(i) {
                *n = p.tf;
            }
        }
        found
    }

    /// The key and counts of `slot`.
    fn counts(&self, slot: u32, tf: Vec<u32>) -> Option<(&Key, Counts)> {
        let at = usize::try_from(slot).ok()?;
        let dl = *self.lengths.get(at)?;
        Some((self.keys.get(at)?, Counts { dl, tf }))
    }

    /// A new slot for `key`; `None` past `u32`.
    fn claim(&mut self, key: &Key) -> Option<u32> {
        let slot = u32::try_from(self.keys.len()).ok()?;
        self.keys.push(key.clone());
        Some(slot)
    }

    /// Post `body`'s terms at `slot`.
    fn learn(&mut self, slot: u32, body: &str) {
        let mut tokens = token::tokens(body);
        self.lengths.push(clamp(tokens.len()));
        tokens.sort_unstable();
        for run in tokens.chunk_by(|a, b| a == b) {
            let Some(term) = run.first() else { continue };
            let p = Posting {
                slot,
                tf: clamp(run.len()),
            };
            match self.terms.get_mut(term) {
                Some(list) => list.push(p),
                None => drop(self.terms.insert(term.clone(), vec![p])),
            }
        }
    }

    /// Drop the postings of the row at `key`, whose body was `old`.
    fn forget(&mut self, key: &Key, old: &Observation) {
        let keys = &self.keys;
        let at = |p: &Posting| {
            let slot = usize::try_from(p.slot).ok();
            slot.and_then(|s| keys.get(s)) == Some(key)
        };
        for term in token::terms(&old.body) {
            let Some(list) = self.terms.get_mut(&term) else {
                continue;
            };
            list.retain(|p| !at(p));
            if list.is_empty() {
                self.terms.remove(&term);
            }
        }
    }
}

impl Index for Postings {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn insert(
        &mut self,
        key: &Key,
        row: &Observation,
        replaced: Option<&Observation>,
    ) {
        if let Some(old) = replaced {
            self.forget(key, old);
        }
        match self.claim(key) {
            Some(slot) => self.learn(slot, &row.body),
            None => self.full = true,
        }
    }

    fn bytes(&self) -> u64 {
        let entry = size_of::<(String, Vec<Posting>)>().saturating_add(1);
        let lists = self.terms.iter().map(|(t, list)| {
            let posts = list.capacity().saturating_mul(size_of::<Posting>());
            t.capacity().saturating_add(posts)
        });
        let ids = self.keys.iter().map(|(_, id)| id.capacity());
        let parts = [
            self.terms.capacity().saturating_mul(entry),
            self.keys.capacity().saturating_mul(size_of::<Key>()),
            self.lengths.capacity().saturating_mul(size_of::<u32>()),
        ];
        let all = parts.into_iter().chain(lists).chain(ids);
        let total = all.fold(0_usize, usize::saturating_add);
        u64::try_from(total).unwrap_or(u64::MAX)
    }
}

/// A tf per term, none found yet.
fn zeros(terms: &[String]) -> Vec<u32> {
    vec![0; terms.len()]
}

/// A count as stored; bodies stay far below `u32::MAX` tokens.
fn clamp(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}
