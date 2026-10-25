//! criterion 0.8's output (benches R1): per bench
//! `<suite>/<fixture>/new/estimates.json`, the median's point estimate,
//! and `new/sample.json`, the raw samples a p95 is taken from.

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::Path;

use tinyjson::JsonValue;

/// 1 bench of a criterion run, in milliseconds a call.
#[derive(Debug, Clone, PartialEq)]
pub struct Bench {
    pub suite: String,
    pub fixture: String,
    pub median_ms: f64,
    pub p95_ms: f64,
    /// When criterion wrote it, `YYYY-MM-DD`.
    pub date: String,
}

/// The date a file was written, `YYYY-MM-DD`.
pub type DateOf = fn(&Path) -> Option<String>;

/// The median's point estimate of `estimates.json`, in ms.
#[must_use]
pub fn median_ms(json: &str) -> Option<f64> {
    let top = object(json)?;
    let median: &HashMap<String, JsonValue> = top.get("median")?.get()?;
    let ns: &f64 = median.get("point_estimate")?.get()?;
    Some(ns / 1e6)
}

/// The 95th percentile, by nearest rank, of 1 call's time over the
/// samples of `sample.json` (each sample: `times` / `iters`), in ms.
#[must_use]
pub fn p95_ms(json: &str) -> Option<f64> {
    let top = object(json)?;
    let iters = numbers(top.get("iters")?)?;
    let times = numbers(top.get("times")?)?;
    let pairs = times.iter().zip(&iters).filter(|(_, n)| **n > 0.0);
    let mut each: Vec<f64> = pairs.map(|(t, n)| t / n / 1e6).collect();
    each.sort_by(f64::total_cmp);
    let rank = each
        .len()
        .checked_mul(95)?
        .checked_add(99)?
        .checked_div(100)?;
    each.get(rank.checked_sub(1)?).copied()
}

fn object(json: &str) -> Option<HashMap<String, JsonValue>> {
    let root: JsonValue = json.parse().ok()?;
    let map: &HashMap<String, JsonValue> = root.get()?;
    Some(map.clone())
}

fn numbers(value: &JsonValue) -> Option<Vec<f64>> {
    let list: &Vec<JsonValue> = value.get()?;
    list.iter().map(|v| v.get::<f64>().copied()).collect()
}

/// Every bench under the criterion dir `dir` with both files readable,
/// sorted by suite then fixture, each dated by `date_of`; a dir that is
/// not a bench is skipped.
#[must_use]
pub fn read(dir: &Path, date_of: DateOf) -> Vec<Bench> {
    let mut found: Vec<Bench> = subdirs(dir)
        .into_iter()
        .flat_map(|(suite, path)| {
            subdirs(&path).into_iter().filter_map(move |(fixture, p)| {
                bench(&p.join("new"), suite.clone(), fixture, date_of)
            })
        })
        .collect();
    found.sort_by(|a, b| (&a.suite, &a.fixture).cmp(&(&b.suite, &b.fixture)));
    found
}

/// 1 bench's numbers from its `new/` dir, dated by `estimates.json`.
fn bench(
    new: &Path,
    suite: String,
    fixture: String,
    date_of: DateOf,
) -> Option<Bench> {
    let estimates = new.join("estimates.json");
    let read = |p: &Path| std::fs::read_to_string(p).ok();
    Some(Bench {
        median_ms: median_ms(&read(&estimates)?)?,
        p95_ms: p95_ms(&read(&new.join("sample.json"))?)?,
        date: date_of(&estimates)?,
        suite,
        fixture,
    })
}

/// The dirs in `dir`, by name; none when it cannot be read.
fn subdirs(dir: &Path) -> Vec<(String, std::path::PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
        .collect()
}
