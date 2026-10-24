//! How much of the window fits: its length, and (T4) the estimate from
//! catalog stats and the overflow policies (src/cache C, D16).

#[cfg(test)]
mod tests;

use crate::config::{Span, Unit};

/// 1 day in milliseconds.
pub(super) const DAY: u64 = 86_400_000;

const HOUR: u64 = 3_600_000;

/// A month counts 31 days, so a window never falls short of one.
const MONTH_DAYS: u64 = 31;

/// The window's length in milliseconds.
pub(super) fn window_ms(span: Span) -> u64 {
    let amount = u64::from(span.amount);
    match span.unit {
        Unit::Hours => amount.saturating_mul(HOUR),
        Unit::Days => amount.saturating_mul(DAY),
        Unit::Months => amount.saturating_mul(MONTH_DAYS * DAY),
    }
}
