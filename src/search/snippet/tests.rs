use super::{WIDTH, around};

fn terms(t: &[&str]) -> Vec<String> {
    t.iter().map(|&w| w.to_owned()).collect()
}

#[test]
fn a_short_body_is_its_own_snippet() {
    assert_eq!(
        around("cargo build failed", &terms(&["build"])),
        "cargo build failed"
    );
}

#[test]
fn a_long_body_starts_a_little_before_the_first_match() {
    let body = format!("{} Needle here {}", "x".repeat(200), "y".repeat(200));
    let got = around(&body, &terms(&["needle"]));
    assert_eq!(got.chars().count(), WIDTH);
    assert!(got.starts_with(&"x".repeat(29)), "{got}");
    assert!(got.contains(" Needle here "), "{got}");
}

#[test]
fn no_match_starts_at_the_beginning_and_counts_chars_not_bytes() {
    let body = "\u{142}".repeat(300);
    let got = around(&body, &terms(&["absent"]));
    assert_eq!(got, "\u{142}".repeat(WIDTH));
}
