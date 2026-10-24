use super::{Span, parse_size};
use crate::config::tests::{days, hours, months};

#[test]
fn sizes_parse_kib_mib_gib() {
    assert_eq!(parse_size("4KiB"), Some(4096));
    assert_eq!(parse_size("1MiB"), Some(1 << 20));
    assert_eq!(parse_size("3GiB"), Some(3 << 30));
    for bad in ["4kb", "MiB", "0MiB", "-1MiB", "1.5GiB", "1 MiB", "1TiB"] {
        assert_eq!(parse_size(bad), None, "{bad}");
    }
}

#[test]
fn durations_parse_d_h_mo() {
    assert_eq!(Span::parse("7d"), Some(days(7)));
    assert_eq!(Span::parse("36h"), Some(hours(36)));
    assert_eq!(Span::parse("1mo"), Some(months(1)));
    for bad in ["7w", "0d", "d", "-3d", "+3d", "3 d", "1m", "1.5d", ""] {
        assert_eq!(Span::parse(bad), None, "{bad}");
    }
}

#[test]
fn spans_display_as_written() {
    assert_eq!(days(7).to_string(), "7d");
    assert_eq!(months(2).to_string(), "2mo");
    assert_eq!(hours(5).to_string(), "5h");
}
