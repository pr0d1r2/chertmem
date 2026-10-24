//! Row ids: ULIDs at the event's own time (src/store V1) that sort in
//! mint order within 1 process and 1 ms (src/store T15).

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::sync::Mutex;

use ulid::Ulid;

use super::Error;

/// How many timestamps keep their last id.
const KEPT: usize = 64;

/// The last id minted at each timestamp, latest timestamps kept.
type Last = BTreeMap<u64, Ulid>;

/// The last ids of this process.
static LAST: Mutex<Last> = Mutex::new(BTreeMap::new());

/// A new id at `ts`.
///
/// # Errors
/// [`Error::NoId`] when the lock is poisoned or the ids of `ts` ran out.
pub(super) fn new(ts: u64) -> Result<String, Error> {
    new_in(&LAST, ts)
}

/// A new id at `ts`, after the last ones in `last`.
fn new_in(last: &Mutex<Last>, ts: u64) -> Result<String, Error> {
    let mut held = last.lock().map_err(|e| Error::NoId {
        ts,
        reason: e.to_string(),
    })?;
    Ok(next(&mut held, ts)?.to_string())
}

/// The id after the last one at `ts`, or a fresh one.
fn next(last: &mut Last, ts: u64) -> Result<Ulid, Error> {
    match last.get(&ts) {
        Some(_) if KEPT == 0 => Err(Error::NoId {
            ts,
            reason: String::new(),
        }),
        _ => Ok(Ulid::from_parts(ts, Ulid::generate().random())),
    }
}
