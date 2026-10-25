use super::{Bench, median_ms, p95_ms, read};
use std::fs;
use std::path::{Path, PathBuf};

type Res = Result<(), Box<dyn std::error::Error>>;

const ESTIMATES: &str =
    r#"{"mean":{"point_estimate":9.0},"median":{"point_estimate":81234.0}}"#;

/// 20 samples: sample k ran k iterations taking k * k * 1000 ns, so 1
/// call of sample k took k microseconds.
fn samples() -> String {
    let iters: Vec<String> = (1..=20).map(|k| format!("{k}.0")).collect();
    let square = |k: u32| k.saturating_mul(k).saturating_mul(1000);
    let times: Vec<String> =
        (1..=20).map(|k| format!("{}.0", square(k))).collect();
    format!(
        r#"{{"sampling_mode":"Linear","iters":[{}],"times":[{}]}}"#,
        iters.join(","),
        times.join(",")
    )
}

fn scratch(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir()
        .join(format!("chertmem-dev-{}-crit-{name}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

const DAY: super::DateOf = |_| Some("2026-10-25".to_owned());

#[test]
fn the_median_is_criterions_point_estimate_in_ms() {
    assert_eq!(median_ms(ESTIMATES), Some(0.081_234));
    assert_eq!(median_ms("{}"), None);
    assert_eq!(median_ms("[1]"), None);
    assert_eq!(median_ms("not json"), None);
}

#[test]
fn the_p95_is_the_nearest_rank_of_1_calls_time() {
    // ceil(0.95 * 20) = rank 19: 19 microseconds.
    assert_eq!(p95_ms(&samples()), Some(0.019));
    let one = r#"{"iters":[2.0],"times":[4000.0]}"#;
    assert_eq!(p95_ms(one), Some(0.002));
    assert_eq!(p95_ms(r#"{"iters":[],"times":[]}"#), None);
    assert_eq!(p95_ms(r#"{"iters":["x"],"times":[1.0]}"#), None);
}

fn plant(dir: &Path, suite: &str, fixture: &str) -> Res {
    let new = dir.join(suite).join(fixture).join("new");
    fs::create_dir_all(&new)?;
    fs::write(new.join("estimates.json"), ESTIMATES)?;
    fs::write(new.join("sample.json"), samples())?;
    Ok(())
}

#[test]
fn every_bench_with_both_files_is_read_sorted() -> Res {
    let dir = scratch("read")?;
    plant(&dir, "b_suite", "1k-p52")?;
    plant(&dir, "a_suite", "10k-p52")?;
    fs::create_dir_all(dir.join("a_suite/1k-p1/new"))?;
    fs::write(dir.join("chert-suites.tsv"), "a_suite\t-\n")?;
    let found = read(&dir, DAY);
    let names: Vec<(&str, &str)> = found
        .iter()
        .map(|b| (b.suite.as_str(), b.fixture.as_str()))
        .collect();
    assert_eq!(names, [("a_suite", "10k-p52"), ("b_suite", "1k-p52")]);
    assert_eq!(found.first().map(|b| b.date.as_str()), Some("2026-10-25"));
    Ok(())
}

#[test]
fn a_missing_dir_or_date_reads_as_nothing() -> Res {
    let dir = scratch("undated")?;
    plant(&dir, "s", "1k-p52")?;
    assert!(read(&dir, |_| None).is_empty());
    assert_eq!(read(&dir.join("absent"), DAY), Vec::<Bench>::new());
    Ok(())
}
