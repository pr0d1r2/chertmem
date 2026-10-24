use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Mutex;

use ulid::Ulid;

use super::{KEPT, Last, new, new_in, next};
use crate::store::Error;
use crate::store::tests::Test;

const TS: u64 = 1_709_211_600_000;

#[test]
fn ids_are_ulids_at_the_event_time() -> Test {
    let (a, b) = (new(TS)?, new(TS)?);
    assert_ne!(a, b);
    assert_eq!(Ulid::from_string(&a)?.timestamp_ms(), TS);
    // 10 chars of time: same millisecond, same first 10 chars.
    assert_eq!((a.len(), a.get(..10)), (26, b.get(..10)));
    Ok(())
}

/// The largest step from 1 id of a ms to the next: 2^70.
const STEP: u128 = 0x40_0000_0000_0000_0000;

/// How far `b` is past `a` in their random parts.
fn step(a: Ulid, b: Ulid) -> Option<u128> {
    b.random().checked_sub(a.random())
}

#[test]
fn the_next_id_of_a_millisecond_steps_up_by_at_most_2_70() -> Test {
    let mut last = Last::new();
    let a = next(&mut last, TS)?;
    let b = next(&mut last, TS)?;
    let up = step(a, b);
    assert!(up.is_some_and(|n| (1..=STEP).contains(&n)), "{up:?}");
    assert_eq!(b.timestamp_ms(), TS);
    Ok(())
}

#[test]
fn a_fresh_id_leaves_room_for_a_long_run() -> Test {
    // Below 2^78: >= 768 steps of 2^70 fit before 2^80.
    for ts in 0..64 {
        let fresh = next(&mut Last::new(), ts)?;
        assert!(fresh.random() < 1 << 78, "{fresh}");
    }
    Ok(())
}

#[test]
fn another_millisecond_keeps_its_own_time_and_run() -> Test {
    let mut last = Last::new();
    let a = next(&mut last, TS)?;
    let b = next(&mut last, TS - 1)?;
    let c = next(&mut last, TS)?;
    assert_eq!(b.timestamp_ms(), TS - 1);
    // Interleaved milliseconds still continue their own run.
    let up = step(a, c);
    assert!(up.is_some_and(|n| (1..=STEP).contains(&n)), "{up:?}");
    Ok(())
}

#[test]
fn a_millisecond_out_of_ids_is_an_error_not_a_panic() {
    let mut last = Last::new();
    last.insert(TS, Ulid::from_parts(TS, u128::MAX));
    let got = next(&mut last, TS).err();
    assert!(matches!(got, Some(Error::NoId { ts: TS, .. })), "{got:?}");
}

#[test]
fn only_the_latest_milliseconds_are_kept() -> Test {
    let mut last = Last::new();
    for ts in 0..=u64::try_from(KEPT)? {
        next(&mut last, ts)?;
    }
    assert_eq!(last.len(), KEPT);
    assert!(!last.contains_key(&0));
    Ok(())
}

#[test]
fn a_poisoned_lock_is_an_error_not_a_panic() {
    let last = Mutex::new(BTreeMap::new());
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _held = last.lock();
        resume_unwind(Box::new("poison"));
    }));
    let got = new_in(&last, TS).err();
    assert!(matches!(got, Some(Error::NoId { ts: TS, .. })), "{got:?}");
}
