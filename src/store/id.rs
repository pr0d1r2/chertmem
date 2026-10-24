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

/// The largest random part: 80 bits.
const MAX_RANDOM: u128 = 0xFFFF_FFFF_FFFF_FFFF_FFFF;
/// A fresh random part's mask: below 2^78, so >= 768 steps fit.
const FRESH: u128 = 0x3FFF_FFFF_FFFF_FFFF_FFFF;
/// A step's mask: a step is 1 + this masked, in [1, 2^70].
const STEP: u128 = 0x3F_FFFF_FFFF_FFFF_FFFF;

/// The id after the last one at `ts`, or a fresh one; the earliest
/// timestamp goes once more than [`KEPT`] are held.
fn next(last: &mut Last, ts: u64) -> Result<Ulid, Error> {
    let random = match last.get(&ts) {
        Some(prev) => after(prev.random(), ts)?,
        None => Ulid::generate().random() & FRESH,
    };
    let id = Ulid::from_parts(ts, random);
    last.insert(ts, id);
    if last.len() > KEPT {
        last.pop_first();
    }
    Ok(id)
}

/// The random part after `prev`: a random step in [1, 2^70], not +1,
/// so consecutive ids differ near char 13 and their refs stay short
/// (root D28). Past 80 bits is [`Error::NoId`]: moving to the next ms
/// would leave the event's own time (src/store V1).
fn after(prev: u128, ts: u64) -> Result<u128, Error> {
    let step = (Ulid::generate().random() & STEP).saturating_add(1);
    let random = prev.checked_add(step).filter(|r| *r <= MAX_RANDOM);
    random.ok_or_else(|| Error::NoId {
        ts,
        reason: "the ids of this ms ran out".to_owned(),
    })
}
