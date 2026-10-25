//! The harness, criterion (benches T1, R1): how long each bench runs.

#[cfg(test)]
mod tests;

use std::time::Duration;

use criterion::Criterion;

/// How long criterion samples 1 bench.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Timing {
    pub(crate) samples: usize,
    pub(crate) warm_up: Duration,
    pub(crate) measure: Duration,
}

/// On push (root V10): criterion's least sample count, short windows,
/// so a bench costs about 1.3 s after its fixture is built.
pub(crate) const PUSH: Timing =
    Timing::new(10, Duration::from_millis(300), Duration::from_secs(1));

impl Timing {
    pub(crate) const fn new(
        samples: usize,
        warm_up: Duration,
        measure: Duration,
    ) -> Self {
        Self {
            samples,
            warm_up,
            measure,
        }
    }
}

// Criterion refuses fewer than 10 samples.
const _: () = assert!(PUSH.samples >= 10);

/// `c` sampling as `t` says, without plots.
pub(crate) fn configured(c: Criterion, t: &Timing) -> Criterion {
    c.sample_size(t.samples)
        .warm_up_time(t.warm_up)
        .measurement_time(t.measure)
        .without_plots()
}
