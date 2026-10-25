//! `benches/baseline.toml` (benches C, V3): 1 row per suite and
//! fixture, with its median, p95, tolerance, machine, date and recount
//! command. `bench-record` writes it; `bench-gate` and `perf-doc` read
//! it.

#[cfg(test)]
mod tests;

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
        (&self.machine, &self.os)
    }
}

/// `macOS 26.0` -> `macOS`.
#[must_use]
pub fn os_name(os: &str) -> &str {
    os
}

/// The comment the file starts with.
pub const HEADER: &str = "";

/// `ms` as written: 4 or 5 significant digits, by size.
#[must_use]
pub fn ms(ms: f64) -> String {
    let _ = ms;
    String::new()
}

/// `ms` rounded as [`ms`] writes it, so a row reads back unchanged.
#[must_use]
pub fn round(value: f64) -> f64 {
    value
}

/// The file's text for `rows`, in the order given.
#[must_use]
pub fn render(rows: &[Row]) -> String {
    let _ = rows;
    String::new()
}

/// The rows of the file's text.
///
/// # Errors
/// Text that is not TOML, or a row missing a key or holding a wrong
/// type, named by its place.
pub fn parse(text: &str) -> Result<Vec<Row>, String> {
    let _ = text;
    Ok(Vec::new())
}
