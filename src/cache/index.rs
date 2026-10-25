//! The hook a keyword index hangs on the window (src/cache T6, R6): the
//! window feeds it every row it adds or replaces, under the lock that
//! guards the rows, so rows and index never disagree. The index itself
//! lives in `src/search` (R5).

#[cfg(test)]
mod tests;

use std::any::Any;

use crate::store::Observation;

/// A row's place in the window: `(ts, id)`, the store's order (R1).
pub type Key = (u64, String);

/// An index kept beside the window's rows. Its owner reads it back
/// through [`super::Handle::with_index`], as `&dyn Any` to its own
/// type.
pub trait Index: Any + Send {
    /// Forget every row: a window begins (warm-up, generation reload).
    fn reset(&mut self);

    /// `row` is now at `key`; `replaced` is the row it took the place
    /// of, whose terms the index drops.
    fn insert(
        &mut self,
        key: &Key,
        row: &Observation,
        replaced: Option<&Observation>,
    );

    /// The bytes it holds, counted toward `cache.max_bytes` (V4).
    fn bytes(&self) -> u64;
}

/// No index: what a cache started without one keeps.
pub(super) struct Rowless;

impl Index for Rowless {
    fn reset(&mut self) {}

    fn insert(&mut self, _: &Key, _: &Observation, _: Option<&Observation>) {}

    fn bytes(&self) -> u64 {
        0
    }
}
