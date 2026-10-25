//! `benches/baseline.toml` (benches C, V3): 1 row per suite and
//! fixture, with its median, p95, tolerance, machine, date and recount
//! command. `bench-record` writes it; `bench-gate` and `perf-doc` read
//! it.

#[cfg(test)]
mod tests;

use std::fmt::Write as _;

use toml::{Table, Value};

/// 1 measured bench.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub suite: String,
    /// The fixture's label, `1k-p52`.
    pub fixture: String,
    /// The root C target the suite measures; `None` when it has none.
    pub target: Option<String>,
    pub median_ms: f64,
    pub p95_ms: f64,
    /// A median over `median_ms x (1 + tolerance)` is a regression.
    pub tolerance: f64,
    /// The CPU model.
    pub machine: String,
    /// The OS name and version, `macOS 26.0`.
    pub os: String,
    /// When criterion wrote the numbers, `YYYY-MM-DD` (UTC).
    pub date: String,
    pub recount: String,
}

impl Row {
    /// The machine class a comparison holds on: the CPU model and the
    /// OS name, its version left out.
    #[must_use]
    pub fn class(&self) -> (&str, &str) {
        (&self.machine, os_name(&self.os))
    }
}

/// `macOS 26.0` -> `macOS`.
#[must_use]
pub fn os_name(os: &str) -> &str {
    os.split_whitespace().next().unwrap_or(os)
}

/// The comment the file starts with.
pub const HEADER: &str = "\
# The latency baseline (benches C, T4): 1 row per suite and fixture.
# Written by `cargo run -q -p chertmem-dev -- bench-record` from a
# criterion run; updated only by its own reviewed commit. A tolerance is
# kept across records, so it is tuned here by hand.
";

/// `ms` as written: 4 or 5 significant digits, by size.
#[must_use]
pub fn ms(ms: f64) -> String {
    let places = [(100.0, 1), (10.0, 2), (1.0, 3), (0.1, 4)];
    let digits = places.iter().find(|(at, _)| ms >= *at).map_or(5, |p| p.1);
    format!("{ms:.digits$}")
}

/// `ms` rounded as [`ms`] writes it, so a row reads back unchanged.
#[must_use]
pub fn round(value: f64) -> f64 {
    ms(value).parse().unwrap_or(value)
}

/// The file's text for `rows`, in the order given.
#[must_use]
pub fn render(rows: &[Row]) -> String {
    rows.iter().fold(HEADER.to_owned(), |mut out, r| {
        push_row(&mut out, r);
        out
    })
}

fn push_row(out: &mut String, r: &Row) {
    out.push_str("\n[[row]]\n");
    field(out, "suite", &r.suite);
    field(out, "fixture", &r.fixture);
    if let Some(target) = &r.target {
        field(out, "target", target);
    }
    let (median, p95) = (ms(r.median_ms), ms(r.p95_ms));
    let _ = writeln!(out, "median_ms = {median}\np95_ms = {p95}");
    let _ = writeln!(out, "tolerance = {:?}", r.tolerance);
    for (key, value) in [("machine", &r.machine), ("os", &r.os)] {
        field(out, key, value);
    }
    field(out, "date", &r.date);
    field(out, "recount", &r.recount);
}

/// `key = "value"`, the value a TOML basic string.
fn field(out: &mut String, key: &str, value: &str) {
    let quoted = value.replace('\\', "\\\\").replace('"', "\\\"");
    let _ = writeln!(out, "{key} = \"{quoted}\"");
}

/// The rows of the file's text.
///
/// # Errors
/// Text that is not TOML, or a row missing a key or holding a wrong
/// type, named by its place.
pub fn parse(text: &str) -> Result<Vec<Row>, String> {
    let table: Table = text.parse().map_err(|e| format!("not TOML: {e}"))?;
    let Some(rows) = table.get("row") else {
        return Ok(Vec::new());
    };
    let rows = rows.as_array().ok_or("`row` is not an array of tables")?;
    let numbered = (1_usize..).zip(rows);
    numbered
        .map(|(n, v)| row(v).map_err(|e| format!("row {n}: {e}")))
        .collect()
}

fn row(value: &Value) -> Result<Row, String> {
    let t = value.as_table().ok_or("not a table")?;
    Ok(Row {
        suite: text(t, "suite")?,
        fixture: text(t, "fixture")?,
        target: t.get("target").map(|_| text(t, "target")).transpose()?,
        median_ms: number(t, "median_ms")?,
        p95_ms: number(t, "p95_ms")?,
        tolerance: number(t, "tolerance")?,
        machine: text(t, "machine")?,
        os: text(t, "os")?,
        date: text(t, "date")?,
        recount: text(t, "recount")?,
    })
}

fn text(t: &Table, key: &str) -> Result<String, String> {
    let value = t.get(key).and_then(Value::as_str);
    value
        .map(str::to_owned)
        .ok_or_else(|| format!("`{key}` is not text"))
}

/// A finite, non-negative number; an integer is taken as one.
fn number(t: &Table, key: &str) -> Result<f64, String> {
    let value = t.get(key).and_then(|v| v.as_float().or_else(|| whole(v)));
    value
        .filter(|n| n.is_finite() && *n >= 0.0)
        .ok_or_else(|| format!("`{key}` is not a number of at least 0"))
}

fn whole(v: &Value) -> Option<f64> {
    let n = u32::try_from(v.as_integer()?).ok()?;
    Some(f64::from(n))
}
