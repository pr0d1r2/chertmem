use super::{MAX_TS, bounds, bucket_name, civil, days_from_civil};
use crate::config::{Span, Unit};

const DAY: u64 = 86_400_000;
const HOUR: u64 = 3_600_000;

const fn span(amount: u32, unit: Unit) -> Span {
    Span { amount, unit }
}

/// 2024-02-29T13:00:00Z.
const LEAP_NOON: u64 = 1_709_211_600_000;
/// 2024-02-01 and 2024-03-01, 00:00Z.
const FEB: u64 = 1_706_745_600_000;
const MAR: u64 = 1_709_251_200_000;

#[test]
fn civil_and_days_from_civil_agree_every_first_of_month() {
    for days in 0..40_000 {
        let (y, m, d) = civil(days);
        assert!((1..=12).contains(&m) && (1..=31).contains(&d));
        if d == 1 {
            assert_eq!(days_from_civil(y, m), days, "{y}-{m}");
        }
    }
}

#[test]
fn civil_knows_known_dates() {
    assert_eq!(civil(0), (1970, 1, 1));
    assert_eq!(civil(LEAP_NOON / DAY), (2024, 2, 29));
    assert_eq!(civil(MAR / DAY), (2024, 3, 1));
}

#[test]
fn days_and_hours_align_to_the_epoch() {
    let week = bounds(span(7, Unit::Days), 0);
    assert_eq!(week, Some((0, 7 * DAY)));
    let t0 = LEAP_NOON - LEAP_NOON % (6 * HOUR);
    let six = bounds(span(6, Unit::Hours), LEAP_NOON);
    assert_eq!(six, Some((t0, t0 + 6 * HOUR)));
    let day = bounds(span(1, Unit::Days), LEAP_NOON);
    assert_eq!(day, Some((LEAP_NOON - 13 * HOUR, MAR)));
}

#[test]
fn months_follow_the_calendar() {
    let one = bounds(span(1, Unit::Months), LEAP_NOON);
    assert_eq!(one, Some((FEB, MAR)));
    // 2024-01-01 to 2024-04-01: quarters start in January.
    let quarter = bounds(span(3, Unit::Months), LEAP_NOON);
    assert_eq!(quarter, Some((1_704_067_200_000, 1_711_929_600_000)));
}

#[test]
fn a_bucket_holds_its_timestamp() {
    let spans = [span(7, Unit::Days), span(5, Unit::Hours)];
    for s in spans.into_iter().chain([span(2, Unit::Months)]) {
        for ts in [0, 1, LEAP_NOON, MAX_TS] {
            let (t0, t1) = bounds(s, ts).unwrap_or_default();
            assert!(t0 <= ts && ts < t1, "{s} {ts}");
        }
    }
}

#[test]
fn past_48_bits_or_zero_length_has_no_bucket() {
    assert_eq!(bounds(span(7, Unit::Days), MAX_TS + 1), None);
    assert_eq!(bounds(span(0, Unit::Days), 5), None);
    assert_eq!(bounds(span(0, Unit::Months), 5), None);
}

#[test]
fn names_carry_start_and_length() {
    assert_eq!(bucket_name(span(7, Unit::Days), 0), "19700101-7d");
    let six = bucket_name(span(6, Unit::Hours), LEAP_NOON - HOUR);
    assert_eq!(six, "20240229T12-6h");
    assert_eq!(bucket_name(span(1, Unit::Months), FEB), "202402-1mo");
}
