//! How much of the window fits `cache.max_bytes`: an estimate from
//! catalog stats before the load, and the overflow policies (src/cache
//! C, D16).

#[cfg(test)]
mod tests;

use std::fmt;

use crate::config::{self, Span, Unit};
use crate::store::Partition;

/// 1 day in milliseconds.
pub(super) const DAY: u64 = 86_400_000;

const HOUR: u64 = 3_600_000;

/// A month counts 31 days, so a window never falls short of one.
const MONTH_DAYS: u64 = 31;

/// The memory 1 row takes beyond its body (src/cache R1): the
/// `Observation` inline (200 B on 64-bit), its `(ts, id)` key (32 B),
/// 8 heap strings (ref, id twice, project, session, ide, kind,
/// identity: ~200 B with allocator rounding) and B-tree node slack,
/// ~500 B; rounded up so the estimate errs high.
pub(super) const ROW_OVERHEAD: u64 = 640;

/// What `cache plan` prints (src/cache I): the window's estimate, the
/// limit, and how many of its newest days are kept in memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    /// Bytes the whole window would take.
    pub estimate: u64,
    /// `cache.max_bytes`.
    pub limit: u64,
    /// 0 when the cache is off and every query reads the disk.
    pub days_kept: u64,
}

impl fmt::Display for Plan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            estimate,
            limit,
            days_kept,
        } = self;
        write!(
            f,
            "estimate {estimate} bytes, limit {limit} bytes, \
             {days_kept} days kept"
        )
    }
}

/// A plan and where the window starts; `since = None`: disk only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Fit {
    pub(super) plan: Plan,
    pub(super) since: Option<u64>,
}

impl Fit {
    /// The estimate is over the limit.
    pub(super) fn over(&self) -> bool {
        self.plan.estimate > self.plan.limit
    }
}

/// The window's length in milliseconds.
pub(super) fn window_ms(span: Span) -> u64 {
    let amount = u64::from(span.amount);
    match span.unit {
        Unit::Hours => amount.saturating_mul(HOUR),
        Unit::Days => amount.saturating_mul(DAY),
        Unit::Months => amount.saturating_mul(MONTH_DAYS * DAY),
    }
}

/// The window in whole days, a part day counting as 1.
pub(super) fn days(span: Span) -> u64 {
    window_ms(span).div_ceil(DAY)
}

/// The bytes a partition's rows take in memory.
fn cost(p: &Partition) -> u64 {
    let rows = p.stats.rows.saturating_mul(ROW_OVERHEAD);
    p.stats.bytes.saturating_add(rows)
}

/// The share of `p`'s cost from `since` on, its rows taken as spread
/// evenly over the part of `[t0, t1)` already past at `now`.
fn share(p: &Partition, since: u64, now: u64) -> u64 {
    let end = p.t1.min(now);
    if since <= p.t0 || end <= p.t0 {
        return cost(p);
    }
    let kept = u128::from(end.saturating_sub(since));
    let whole = u128::from(cost(p)).saturating_mul(kept);
    let part = whole.checked_div(u128::from(end.saturating_sub(p.t0)));
    part.map_or(0, |b| u64::try_from(b).unwrap_or(u64::MAX))
}

/// The bytes of every row from `since` on.
pub(super) fn estimate(parts: &[Partition], since: u64, now: u64) -> u64 {
    let shares = parts.iter().map(|p| share(p, since, now));
    shares.fold(0, u64::saturating_add)
}

/// Fit the window of `cfg` ending at `now` to its limit (D16).
pub(super) fn fit(parts: &[Partition], cfg: &config::Cache, now: u64) -> Fit {
    // RED stub: the whole window is kept, whatever the estimate.
    let start = now.saturating_sub(window_ms(cfg.window));
    Fit {
        plan: Plan {
            estimate: estimate(parts, start, now),
            limit: cfg.max_bytes,
            days_kept: days(cfg.window),
        },
        since: Some(start),
    }
}

/// The overflow warning: it names the estimate and the limit (V4,
/// root V11) and what is kept.
pub(super) fn overflow(plan: &Plan, cfg: &config::Cache) -> String {
    let (estimate, limit) = (plan.estimate, plan.limit);
    let kept = if plan.days_kept == 0 {
        "the cache is off, queries read the disk".to_owned()
    } else {
        let (kept, days) = (plan.days_kept, days(cfg.window));
        format!("keeping the newest {kept} of {days} days")
    };
    format!(
        "chert: cache estimate {estimate} bytes is over cache.max_bytes \
         {limit} bytes; {kept} (raise cache.max_bytes or shorten \
         cache.window)"
    )
}

/// A warning when `max` is over 25% of `available` (src/cache C);
/// `None` when it is not, or the figure is unknown.
pub(super) fn memory(_max: u64, _available: Option<u64>) -> Option<String> {
    // RED stub: never warns.
    None
}
