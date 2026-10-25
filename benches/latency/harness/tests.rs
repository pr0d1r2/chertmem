// Only `#[test]` fns here, imports inside them: `cargo bench` builds
// this module with `--cfg test` and strips every test (lib.rs).

#[test]
fn a_push_run_stays_short() {
    // benches T1: at most 2 s a bench, so the grid's ~80 benches fit a
    // push; criterion's least sample count is asserted at compile time.
    let each = super::PUSH.warm_up.saturating_add(super::PUSH.measure);
    assert!(each <= std::time::Duration::from_secs(2), "{each:?}");
}

#[test]
fn criterion_exports_every_median_as_json() -> crate::fixture::Res<()> {
    // benches R1: the gate (T4) reads `<group>/<bench>/new/estimates.json`.
    let out = std::env::temp_dir()
        .join(format!("chert-bench-{}-export", std::process::id()));
    let ms = std::time::Duration::from_millis(1);
    let quick = super::Timing::new(super::PUSH.samples, ms, ms);
    let mut c = super::configured(criterion::Criterion::default(), &quick)
        .output_directory(&out);
    c.bench_function("noop", |b| b.iter(|| 1));
    let estimates =
        std::fs::read_to_string(out.join("noop/new/estimates.json"));
    std::fs::remove_dir_all(&out)?;
    let value: serde_json::Value = serde_json::from_str(&estimates?)?;
    let median = value.pointer("/median/point_estimate");
    assert!(median.is_some_and(serde_json::Value::is_f64), "{value}");
    Ok(())
}

#[test]
fn a_suite_says_what_it_measures_against_which_target() {
    let said: Vec<String> =
        crate::suites::ALL.iter().map(super::measures).collect();
    let get = "get_observations, target get_observations < 2 ms";
    assert!(said.iter().any(|s| s == get));
    assert!(said.iter().any(|s| s == "partition scan, no root C target"));
}

#[test]
fn an_error_exits_2_and_a_code_passes_through() {
    assert_eq!(super::exit(Err("broken".into())), 2);
    assert_eq!(super::exit(Ok(3)), 3);
}

#[test]
fn every_ready_suite_runs_under_criterion() -> crate::fixture::Res<()> {
    use crate::suites::{ALL, Suite};
    let out = crate::fixture::Scratch::new("harness-run")?;
    let ms = std::time::Duration::from_millis(1);
    let quick = super::Timing::new(super::PUSH.samples, ms, ms);
    let c = super::configured(criterion::Criterion::default(), &quick);
    // The semantic suite needs OLLAMA_HOST (ops::search tests); seed
    // 9001 keeps this fixture, under target/, apart from every other.
    let mut picked = ALL.to_vec();
    picked.retain(|s| s.id != "semantic_rerank");
    let tiny = |_: &Suite| vec![crate::fixture::Spec::new(9001, 120, 2)];
    assert_eq!(super::run(c.output_directory(out.path()), &picked, tiny), 0);
    let groups = std::fs::read_dir(out.path())?.count();
    assert_eq!(groups, picked.len().saturating_sub(1), "1 is pending");
    Ok(())
}
