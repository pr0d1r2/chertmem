//! JSONL export: 1 observation per line, its `ref` as `chert:<ref>`,
//! its body exactly as stored, so nothing redacted can come back (root
//! I, V8).

#[cfg(test)]
mod tests;

use std::io::Write;

use super::Error;
use crate::store::{Filter, Observation, Store};

/// Write every observation `filter` wants to `out`, 1 JSON object per
/// line in the store's (ts, id) order; the number of lines.
///
/// # Errors
/// A store that cannot be read (it needs a reading open), or a failed
/// write.
pub fn export(
    store: &mut Store,
    filter: &Filter,
    out: &mut impl Write,
) -> Result<usize, Error> {
    let _ = (store, filter, out, line, Error::io);
    Ok(0)
}

/// 1 observation as 1 JSON line.
pub(super) fn line(observation: &Observation) -> String {
    let _ = observation;
    String::new()
}
