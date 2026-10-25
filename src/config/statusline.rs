//! `statusline.*`: the thresholds `chert statusline` colors its badge
//! by (src/config T7, src/cli T3). RED stub: the defaults only.

#[cfg(test)]
mod tests;

use super::Error;
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

impl Statusline {
    #[allow(clippy::unnecessary_wraps, reason = "RED stub")]
    pub(super) fn build(_src: &Source) -> Result<Self, Error> {
        Ok(Self {
            ms_green: 10,
            ms_amber: 50,
            size_green: 1 << 30,
            size_amber: 5 << 30,
            fit_amber: 80,
        })
    }
}
