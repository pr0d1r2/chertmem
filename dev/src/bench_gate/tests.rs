use super::verdict;
use crate::baseline::Row;
use crate::criterion::Bench;

fn row(fixture: &str, median_ms: f64, machine: &str) -> Row {
    Row {
        suite: "get_observations".to_owned(),
        fixture: fixture.to_owned(),
        target: Some("get_observations".to_owned()),
        median_ms,
        p95_ms: median_ms,
        tolerance: 0.5,
        machine: machine.to_owned(),
        os: "TestOS 1.0".to_owned(),
        date: "2026-10-25".to_owned(),
        recount: "cargo bench -- '^get_observations/1k-p52$'".to_owned(),
    }
}

fn now(fixture: &str, median_ms: f64) -> Bench {
    Bench {
        suite: "get_observations".to_owned(),
        fixture: fixture.to_owned(),
        median_ms,
        p95_ms: median_ms,
        date: "2026-10-26".to_owned(),
    }
}

const CLASS: (&str, &str) = ("CPU", "TestOS");

#[test]
fn a_median_over_baseline_times_1_plus_tolerance_regressed() {
    // dev V6: the planted violation, 1.6 > 1.0 x 1.5, and its clean twin.
    let base = [row("1k-p52", 1.0, "CPU")];
    let slow = verdict(&base, CLASS, &[], &[now("1k-p52", 1.6)]);
    assert_eq!(slow.compared, 1);
    let line = slow.regressions.first().cloned().unwrap_or_default();
    assert!(
        line.starts_with("get_observations/1k-p52: median 1.600 ms > 1.500 ms")
    );
    assert!(line.contains("recount: cargo bench"), "{line}");
    let fine = verdict(&base, CLASS, &[], &[now("1k-p52", 1.4)]);
    assert!(fine.regressions.is_empty());
}

#[test]
fn only_rows_of_the_same_machine_class_compare() {
    let base = [row("1k-p52", 1.0, "Other CPU")];
    let v = verdict(&base, CLASS, &[], &[now("1k-p52", 9.0)]);
    assert_eq!((v.compared, v.regressions.len()), (0, 0));
    let mut newer_os = row("1k-p52", 1.0, "CPU");
    newer_os.os = "TestOS 9.9".to_owned();
    let v = verdict(&[newer_os], CLASS, &[], &[now("1k-p52", 9.0)]);
    assert_eq!(v.regressions.len(), 1, "the OS version is not the class");
}

#[test]
fn named_fixtures_limit_the_gate_and_unmeasured_rows_are_skipped() {
    let base = [row("1k-p52", 1.0, "CPU"), row("1M-p52", 1.0, "CPU")];
    let run = [now("1k-p52", 1.2), now("1M-p52", 9.0)];
    let only = ["1k-p52".to_owned()];
    let v = verdict(&base, CLASS, &only, &run);
    assert_eq!((v.compared, v.regressions.len()), (1, 0));
    let v = verdict(&base, CLASS, &[], &[now("1k-p52", 1.2)]);
    assert_eq!((v.compared, v.regressions.len()), (1, 0));
}
