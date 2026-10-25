use super::{Host, TOLERANCE, fixture_size, index, recount, rows};
use crate::criterion::Bench;

fn bench(suite: &str, fixture: &str, median_ms: f64) -> Bench {
    Bench {
        suite: suite.to_owned(),
        fixture: fixture.to_owned(),
        median_ms,
        p95_ms: median_ms * 2.0,
        date: "2026-10-25".to_owned(),
    }
}

fn host(machine: &str) -> Host {
    Host {
        machine: machine.to_owned(),
        os: "TestOS 2.0".to_owned(),
    }
}

const INDEX: &str = "get_observations\tget_observations\ncross_partition\t-\n";

#[test]
fn the_index_gives_each_suites_target_in_order() {
    let got = index(INDEX);
    let want = [
        (
            "get_observations".to_owned(),
            Some("get_observations".to_owned()),
        ),
        ("cross_partition".to_owned(), None),
    ];
    assert_eq!(got, want);
    assert!(index("no tab here\n").is_empty());
}

#[test]
fn a_run_becomes_rows_by_suite_order_then_size() {
    let ids = "cross_partition/1k-p52 get_observations/100k-p1 \
               get_observations/10k-p52 noop/1k-p1";
    let run: Vec<Bench> = ids
        .split(' ')
        .filter_map(|id| id.split_once('/'))
        .map(|(s, f)| bench(s, f, 1.0))
        .collect();
    let got = rows(&index(INDEX), &run, &host("CPU"), &[]);
    let order: Vec<String> = got
        .iter()
        .map(|r| format!("{}/{}", r.suite, r.fixture))
        .collect();
    let want = "get_observations/10k-p52 get_observations/100k-p1 \
                cross_partition/1k-p52";
    assert_eq!(order.join(" "), want, "noop is not in the index");
}

#[test]
fn a_row_carries_its_target_machine_date_and_recount() -> Result<(), String> {
    let run = [bench("get_observations", "1k-p52", 0.08)];
    let got = rows(&index(INDEX), &run, &host("CPU"), &[]);
    let r = got.first().ok_or("no row")?;
    assert_eq!(r.target.as_deref(), Some("get_observations"));
    assert_eq!((r.machine.as_str(), r.os.as_str()), ("CPU", "TestOS 2.0"));
    assert_eq!(r.date, "2026-10-25");
    assert_eq!(r.recount, recount("get_observations", "1k-p52"));
    assert!((r.tolerance - TOLERANCE).abs() < 1e-9);
    Ok(())
}

#[test]
fn a_rerun_replaces_its_rows_keeps_the_rest_and_the_tolerance() {
    let idx = index(INDEX);
    let first = [bench("cross_partition", "1k-p52", 3.0)];
    let mut before = rows(&idx, &first, &host("CPU"), &[]);
    before.extend(rows(&idx, &first, &host("Other CPU"), &[]));
    for r in &mut before {
        r.tolerance = 0.2;
    }
    let again = [bench("cross_partition", "1k-p52", 4.0)];
    let got = rows(&idx, &again, &host("CPU"), &before);
    let medians: Vec<(&str, f64)> = got
        .iter()
        .map(|r| (r.machine.as_str(), r.median_ms))
        .collect();
    assert_eq!(medians, [("CPU", 4.0), ("Other CPU", 3.0)]);
    assert!(got.iter().all(|r| (r.tolerance - 0.2).abs() < 1e-9));
}

#[test]
fn fixture_labels_read_as_size_and_partitions() {
    assert_eq!(fixture_size("1k-p52"), Some((1000, 52)));
    assert_eq!(fixture_size("1M-p52"), Some((1_000_000, 52)));
    assert_eq!(fixture_size("250-p1"), Some((250, 1)));
    assert_eq!(fixture_size("noop"), None);
    assert_eq!(fixture_size("xk-p1"), None);
}

#[test]
fn numbers_are_rounded_as_the_baseline_writes_them() {
    let run = [bench("get_observations", "1k-p52", 0.081_234)];
    let got = rows(&index(INDEX), &run, &host("CPU"), &[]);
    let pair = got.first().map(|r| (r.median_ms, r.p95_ms));
    assert_eq!(pair, Some((0.081_23, 0.1625)));
}

#[test]
fn the_recount_filters_to_1_bench() {
    let want = "cargo bench -p chertmem --bench latency -- '^s/1k-p52$'";
    assert_eq!(recount("s", "1k-p52"), want);
}
