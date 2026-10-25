//! The badge's values as text, each shortened and colored on its own
//! (src/cli T3, R6). RED stub: nothing shortened, nothing colored.

#[cfg(test)]
mod tests;

/// A value's color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Hue {
    Green,
    Amber,
    Red,
    /// The label while a `chert mcp` of the project runs.
    Live,
    /// The label while none does.
    Down,
}

/// `text` wrapped in `hue` when `color`; plain without a hue.
pub(super) fn paint(text: &str, _hue: Option<Hue>, _color: bool) -> String {
    text.to_owned()
}

/// Green below `green`, amber below `amber`, red at or above it.
pub(super) fn band(_value: u64, _green: u64, _amber: u64) -> Hue {
    Hue::Green
}

/// The count's hue: the cache window's `estimate` against `limit`.
pub(super) fn fit(_estimate: u64, _limit: u64, _amber: u32) -> Hue {
    Hue::Green
}

/// `n` shortened as itok's `shorten`.
pub(super) fn count(n: u64) -> String {
    n.to_string()
}

/// `bytes` in B, KB, MB or GB.
pub(super) fn size(bytes: u64) -> String {
    format!("{bytes}B")
}

/// Whole milliseconds of `us`.
pub(super) fn ms(us: u64) -> u64 {
    us
}
