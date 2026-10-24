use super::{date, number, version};

#[test]
fn dates_try_seconds_then_minutes_then_the_day() {
    assert_eq!(date("2024-01-02T03:04:05", 0), Some(19));
    assert_eq!(date("2024-01-02T03:04:05x", 0), Some(16));
    assert_eq!(date("2024-01-02 03:04x", 0), Some(10));
    assert_eq!(date("2024-01-02x", 0), None);
    assert_eq!(date("2024-1-02", 0), None);
    assert_eq!(date("x2024-01-02", 1), None);
}

#[test]
fn versions_need_major_and_minor() {
    assert_eq!(version("v1.2 x", 0), Some(4));
    assert_eq!(version("1.2.3", 0), Some(5));
    assert_eq!(version("1", 0), None);
    assert_eq!(version("v.1", 0), None);
    assert_eq!(version("1.x", 0), None);
}

#[test]
fn version_suffix_backs_off_to_a_boundary() {
    assert_eq!(version("1.2.3-rc.1.", 0), Some(10));
    assert_eq!(version("1.2.3-.", 0), Some(5));
    assert_eq!(version("1.2+x_y", 0), Some(7));
    assert_eq!(version("1.2-.a", 0), Some(6));
    assert_eq!(version("1.2.3a", 0), Some(3));
    assert_eq!(version("1.2a", 0), None);
}

#[test]
fn numbers_take_a_fraction_when_it_ends_cleanly() {
    assert_eq!(number("3.14 x", 0), Some(4));
    assert_eq!(number("7.", 0), Some(1));
    assert_eq!(number("1.2x", 0), Some(1));
    assert_eq!(number("1.x", 0), Some(1));
    assert_eq!(number("42x", 0), None);
    assert_eq!(number("a1", 1), None);
}
