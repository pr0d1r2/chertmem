//! Targeted retrieval (src/search G): prune partitions, query each in
//! parallel, merge top-k under global term stats; 1 scoring function
//! for cached and disk rows (D17), an optional vector re-rank (D18).

#[cfg(test)]
mod tests;

mod error;
// Read by the engine's query path, which lands in T2.
#[allow(dead_code)]
mod terms;
// Read by the term stats and the engine's query path (T2).
#[allow(dead_code)]
mod token;

pub use error::Error;
