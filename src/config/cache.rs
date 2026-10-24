//! `cache.*` and the window/bucket warning (src/config V2, root C D19).

#[cfg(test)]
mod tests;

use super::Error;
use super::parse::{SIZE_FORMS, SPAN_FORMS, Span, Unit, size, span};
use super::source::Source;

/// `cache.on_overflow` (src/cache C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    Warn,
    Shrink,
    Fail,
}

const OVERFLOWS: &[(&str, Overflow)] = &[
    ("warn", Overflow::Warn),
    ("shrink", Overflow::Shrink),
    ("fail", Overflow::Fail),
];

/// `cache.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cache {
    pub window: Span,
    pub max_bytes: u64,
    pub on_overflow: Overflow,
}

/// Root C D19 defaults.
const WINDOW: Span = Span {
    amount: 28,
    unit: Unit::Days,
};
const MAX_BYTES: u64 = 256 << 20;

impl Cache {
    pub(super) fn build(src: &Source) -> Result<Self, Error> {
        let window = src.get("cache.window", SPAN_FORMS, span)?;
        let max_bytes = src.get("cache.max_bytes", SIZE_FORMS, size)?;
        let on_overflow = src.pick("cache.on_overflow", OVERFLOWS)?;
        Ok(Self {
            window: window.unwrap_or(WINDOW),
            max_bytes: max_bytes.unwrap_or(MAX_BYTES),
            on_overflow: on_overflow.unwrap_or(Overflow::Warn),
        })
    }
}

/// A warning when `window` is not a whole multiple of `bucket`
/// (src/config V2); the load still succeeds.
pub(super) fn window_warning(window: Span, bucket: Span) -> Option<String> {
    if whole_multiple(window, bucket) {
        return None;
    }
    Some(format!(
        "chert: cache.window = \"{window}\" is not a whole multiple of \
         partition.time_bucket = \"{bucket}\" (make the window a whole \
         number of buckets)"
    ))
}

/// Hours and days compare in hours; months only against months, as a
/// month has no fixed length in days.
fn whole_multiple(window: Span, bucket: Span) -> bool {
    match (window.hours(), bucket.hours()) {
        (Some(w), Some(b)) => w.checked_rem(b) == Some(0),
        (None, None) => window.amount.checked_rem(bucket.amount) == Some(0),
        _ => false,
    }
}
