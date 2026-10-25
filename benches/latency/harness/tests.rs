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
fn criterion_exports_every_median_as_json()
-> Result<(), Box<dyn std::error::Error>> {
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
