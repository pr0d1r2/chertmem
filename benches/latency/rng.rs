//! A seeded generator for fixtures: `SplitMix64` (Steele, Lea and Flood,
//! 2014), small enough to keep here instead of a crate (benches T2).

#[cfg(test)]
mod tests;

/// The `SplitMix64` state; the same seed gives the same stream.
#[derive(Debug, Clone)]
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next 64 bits.
    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let z = self.0;
        let z = (z ^ z.wrapping_shr(30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        let z = (z ^ z.wrapping_shr(27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ z.wrapping_shr(31)
    }

    /// A number in `0..n`; 0 when `n` is 0.
    pub(crate) fn below(&mut self, n: u64) -> u64 {
        self.next().checked_rem(n).unwrap_or(0)
    }

    /// A number in `0..n`, small ones likelier: `P(k)` falls about as
    /// `ln(n / k)`, near the Zipf curve of words in text.
    pub(crate) fn skewed(&mut self, n: u64) -> u64 {
        let top = self.below(n);
        self.below(top.saturating_add(1))
    }
}
