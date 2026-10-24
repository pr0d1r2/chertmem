//! Row ids: ULIDs at the event's own time (src/store V1) that sort in
//! mint order within 1 process and 1 ms (src/store T15).

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::sync::Mutex;

use ulid::Ulid;

use super::Error;

/// How many timestamps keep their last id: enough that a run at 1 ts
/// survives 1000s of others minted in between, as a parallel test
/// burst does, in ~160 KiB at most.
const KEPT: usize = 4096;

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

/// The id after the last one at `ts` (its random part + 1, as
/// `ulid::Generator` does), or a fresh one; the earliest timestamp goes
/// once more than [`KEPT`] are held. A random part already at its
/// maximum is [`Error::NoId`]: moving to the next ms would leave the
/// event's own time (src/store V1).
fn next(last: &mut Last, ts: u64) -> Result<Ulid, Error> {
    let id = match last.get(&ts) {
        Some(prev) => prev.increment().map_err(|_| Error::NoId {
            ts,
            reason: "all 2^80 ids of this ms are minted".to_owned(),
        })?,
        None => Ulid::from_parts(ts, Ulid::generate().random()),
    };
    last.insert(ts, id);
    if last.len() > KEPT {
        last.pop_first();
    }
    Ok(id)
}
