//! `statusline.*`: the thresholds `chert statusline` colors its badge
//! by (src/config T7, src/cli T3), as itok's `[rate]` colors its own.

#[cfg(test)]
mod tests;

use toml::Value;

use super::Error;
use super::parse::{POSITIVE, SIZE_FORMS, positive, size};
use super::source::Source;

/// `statusline.*`: below green is green, below amber amber, else red.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Statusline {
    /// Reply latency in ms; default 10.
    pub ms_green: u32,
    /// Default 50.
    pub ms_amber: u32,
    /// The project's bytes; default 1 GiB.
    pub size_green: u64,
    /// Default 5 GiB.
    pub size_amber: u64,
    /// The cache window's estimate as a percent of `cache.max_bytes`
    /// at which the count turns amber; red once over it. Default 80.
    pub fit_amber: u32,
}

/// An integer percent, 1 to 100.
const PERCENT: &str = "an integer from 1 to 100";

impl Statusline {
    pub(super) fn build(src: &Source) -> Result<Self, Error> {
        let ms = |key| src.get(key, POSITIVE, positive);
        let bytes = |key| src.get(key, SIZE_FORMS, size);
        let fit = src.get("statusline.fit_amber", PERCENT, percent)?;
        Ok(Self {
            ms_green: ms("statusline.ms_green")?.unwrap_or(10),
            ms_amber: ms("statusline.ms_amber")?.unwrap_or(50),
            size_green: bytes("statusline.size_green")?.unwrap_or(1 << 30),
            size_amber: bytes("statusline.size_amber")?.unwrap_or(5 << 30),
            fit_amber: fit.unwrap_or(80),
        })
    }
}

fn percent(value: &Value) -> Option<u32> {
    positive(value).filter(|p| *p <= 100)
}
