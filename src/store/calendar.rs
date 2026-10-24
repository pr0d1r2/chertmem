//! Time buckets: `[t0, t1)` of the bucket holding a timestamp, and the
//! bucket's file name (src/store V1, R1).

#[cfg(test)]
mod tests;

use crate::config::{Span, Unit};

/// The largest timestamp a ULID holds: 48 bits of milliseconds.
pub(super) const MAX_TS: u64 = (1 << 48) - 1;

const HOUR_MS: u64 = 3_600_000;
const DAY_MS: u64 = 86_400_000;

/// `[t0, t1)` of the bucket holding `ts`: hours and days aligned to the
/// Unix epoch, months to the calendar (src/store R1). `None` past
/// [`MAX_TS`] or for a zero-length span.
pub(super) fn bounds(span: Span, ts: u64) -> Option<(u64, u64)> {
    if ts > MAX_TS {
        return None;
    }
    let unit = match span.unit {
        Unit::Hours => HOUR_MS,
        Unit::Days => DAY_MS,
        Unit::Months => return months(span.amount, ts),
    };
    let len = u64::from(span.amount).checked_mul(unit)?;
    let t0 = ts.checked_sub(ts.checked_rem(len)?)?;
    Some((t0, t0.checked_add(len)?))
}

/// The calendar bucket of `ts`; months count from 0000-01, so 3 months
/// start in January, April, July and October.
fn months(amount: u32, ts: u64) -> Option<(u64, u64)> {
    let (y, m, _) = civil(ts / DAY_MS);
    let index = y.wrapping_mul(12).wrapping_add(m).wrapping_sub(1);
    let n = u64::from(amount);
    let first = index.wrapping_sub(index.checked_rem(n)?);
    Some((month_start(first), month_start(first.wrapping_add(n))))
}

/// The first millisecond of month `index` (months since 0000-01).
fn month_start(index: u64) -> u64 {
    let (y, m) = (index / 12, index % 12);
    days_from_civil(y, m.wrapping_add(1)).wrapping_mul(DAY_MS)
}

/// `<t0>-<span>`: `YYYYMMDD` for days, `YYYYMMDDTHH` for hours, `YYYYMM`
/// for months, so 2 bucket lengths never share a file.
pub(super) fn bucket_name(span: Span, t0: u64) -> String {
    let (y, m, d) = civil(t0 / DAY_MS);
    let hour = t0 % DAY_MS / HOUR_MS;
    let start = match span.unit {
        Unit::Hours => format!("{y:04}{m:02}{d:02}T{hour:02}"),
        Unit::Days => format!("{y:04}{m:02}{d:02}"),
        Unit::Months => format!("{y:04}{m:02}"),
    };
    format!("{start}-{span}")
}

// The calendar math is Howard Hinnant's `civil_from_days` and
// `days_from_civil`, unsigned: no bucket starts before 1970 and a ULID
// timestamp (48 bits) is under 3.3e6 days, so no step can wrap.

/// (year, month, day) of `days` since 1970-01-01.
pub(super) fn civil(days: u64) -> (u64, u64, u64) {
    let z = days.wrapping_add(719_468);
    let (era, doe) = (z / 146_097, z % 146_097);
    let yoe = year_of_era(doe);
    let doy = doe.wrapping_sub(year_days(yoe));
    let mp = doy.wrapping_mul(5).wrapping_add(2) / 153;
    let start = mp.wrapping_mul(153).wrapping_add(2) / 5;
    let day = doy.wrapping_sub(start).wrapping_add(1);
    let month = (mp.wrapping_add(2) % 12).wrapping_add(1);
    let year = era.wrapping_mul(400).wrapping_add(yoe);
    (year.wrapping_add(u64::from(month <= 2)), month, day)
}

/// The year of an era holding day `doe` (0..146097).
fn year_of_era(doe: u64) -> u64 {
    doe.wrapping_sub(doe / 1460)
        .wrapping_add(doe / 36_524)
        .wrapping_sub(doe / 146_096)
        / 365
}

/// Days before year `yoe` of a 400-year era.
fn year_days(yoe: u64) -> u64 {
    yoe.wrapping_mul(365)
        .wrapping_add(yoe / 4)
        .wrapping_sub(yoe / 100)
}

/// Days since 1970-01-01 of the 1st of `month` in `year` (>= 1970).
pub(super) fn days_from_civil(year: u64, month: u64) -> u64 {
    let y = year.wrapping_sub(u64::from(month <= 2));
    let (era, yoe) = (y / 400, y % 400);
    let mp = if month > 2 {
        month.wrapping_sub(3)
    } else {
        month.wrapping_add(9)
    };
    let doy = mp.wrapping_mul(153).wrapping_add(2) / 5;
    let doe = year_days(yoe).wrapping_add(doy);
    era.wrapping_mul(146_097)
        .wrapping_add(doe)
        .wrapping_sub(719_468)
}
