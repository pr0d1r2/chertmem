//! Each badge value's text and color, apart from any store.

use super::{Hue, band, count, fit, ms, paint, size};

#[test]
fn a_count_shortens_as_itok_by_ceiling() {
    let cases = [
        (0, "0"),
        (99, "99"),
        (100, "1k"),
        (12_300, "13k"),
        (999_000, "999k"),
        (999_001, "1m"),
        (12_300_000, "13m"),
    ];
    for (n, want) in cases {
        assert_eq!(count(n), want, "{n}");
    }
}

#[test]
fn a_size_steps_b_kb_mb_gb_rounded_up_to_a_tenth() {
    let cases = [
        (0, "0B"),
        (512, "512B"),
        (1023, "1023B"),
        (1024, "1KB"),
        (1025, "1.1KB"),
        (3_565_158, "3.4MB"),
        ((1 << 20) - 1, "1MB"),
        (5 << 30, "5GB"),
        (2 << 40, "2048GB"),
        (u64::MAX, "17179869184GB"),
    ];
    for (bytes, want) in cases {
        assert_eq!(size(bytes), want, "{bytes}");
    }
}

#[test]
fn a_latency_rounds_up_to_whole_ms() {
    let max = (u64::MAX, 18_446_744_073_709_552);
    let cases = [(0, 0), (1, 1), (1000, 1), (1001, 2), max];
    for (us, want) in cases {
        assert_eq!(ms(us), want, "{us}");
    }
}

#[test]
fn a_band_is_green_below_green_amber_below_amber_else_red() {
    let cases = [(9, Hue::Green), (10, Hue::Amber), (49, Hue::Amber)];
    for (value, want) in cases.into_iter().chain([(50, Hue::Red)]) {
        assert_eq!(band(value, 10, 50), want, "{value}");
    }
}

#[test]
fn a_fit_is_amber_from_its_percent_and_red_over_the_limit() {
    let cases = [
        (79, Hue::Green),
        (80, Hue::Amber),
        (100, Hue::Amber),
        (101, Hue::Red),
    ];
    for (estimate, want) in cases {
        assert_eq!(fit(estimate, 100, 80), want, "{estimate}");
    }
    assert_eq!(fit(u64::MAX, u64::MAX, 100), Hue::Amber);
}

#[test]
fn paint_wraps_in_the_hue_only_when_coloring() {
    assert_eq!(paint("9ms", Some(Hue::Green), true), "\x1b[32m9ms\x1b[0m");
    assert_eq!(paint("9ms", Some(Hue::Amber), true), "\x1b[33m9ms\x1b[0m");
    assert_eq!(paint("9ms", Some(Hue::Red), true), "\x1b[31m9ms\x1b[0m");
    assert_eq!(paint("9ms", Some(Hue::Red), false), "9ms");
    assert_eq!(paint("0B", None, true), "0B");
}

#[test]
fn the_label_wears_cavemems_256_colors() {
    let live = paint("chert", Some(Hue::Live), true);
    assert_eq!(live, "\x1b[38;5;172mchert\x1b[0m");
    let down = paint("chert", Some(Hue::Down), true);
    assert_eq!(down, "\x1b[38;5;196mchert\x1b[0m");
}
