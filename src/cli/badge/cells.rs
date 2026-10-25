//! The badge's values as text, each shortened and colored on its own
//! (src/cli T3, R6): itok's ceiling and palette for the values,
//! cavemem's 256-color label.

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

impl Hue {
    /// The ANSI SGR that starts it: itok's 32/33/31 for values,
    /// cavemem's 172 (alive) and 196 (down) for the label.
    fn code(self) -> &'static str {
        match self {
            Self::Green => "\x1b[32m",
            Self::Amber => "\x1b[33m",
            Self::Red => "\x1b[31m",
            Self::Live => "\x1b[38;5;172m",
            Self::Down => "\x1b[38;5;196m",
        }
    }
}

/// `text` wrapped in `hue` when `color`; plain without a hue.
pub(super) fn paint(text: &str, hue: Option<Hue>, color: bool) -> String {
    match hue.filter(|_| color) {
        Some(h) => format!("{}{text}\x1b[0m", h.code()),
        None => text.to_owned(),
    }
}

/// Green below `green`, amber below `amber`, red at or above it.
pub(super) fn band(value: u64, green: u64, amber: u64) -> Hue {
    if value < green {
        Hue::Green
    } else if value < amber {
        Hue::Amber
    } else {
        Hue::Red
    }
}

/// The count's hue: the cache window's `estimate` against `limit`,
/// amber from `amber` percent of it, red over it.
pub(super) fn fit(estimate: u64, limit: u64, amber: u32) -> Hue {
    let used = u128::from(estimate).saturating_mul(100);
    let line = u128::from(limit).saturating_mul(u128::from(amber));
    if estimate > limit {
        Hue::Red
    } else if used >= line {
        Hue::Amber
    } else {
        Hue::Green
    }
}

/// `n` shortened as itok's `shorten` (its V108): raw to 99, then `k`
/// to 999000, then `m`, each rounded up so it never understates.
pub(super) fn count(n: u64) -> String {
    match n {
        0..=99 => n.to_string(),
        100..=999_000 => format!("{}k", n.div_ceil(1000)),
        _ => format!("{}m", n.div_ceil(1_000_000)),
    }
}

/// The units past bytes, 1024 apart.
const UNITS: [(&str, u64); 3] =
    [("GB", 1 << 30), ("MB", 1 << 20), ("KB", 1 << 10)];

/// `bytes` in B, or the largest of KB, MB, GB it reaches, rounded up
/// to a tenth and the `.0` dropped. A unit is reached once the one
/// below it would round up to 1024.0, so `1024KB` is never printed.
pub(super) fn size(bytes: u64) -> String {
    let wide = u128::from(bytes).saturating_mul(10);
    let tenths = |unit: u64| wide.div_ceil(u128::from(unit));
    let reached = UNITS.iter().find(|&&(_, u)| tenths(u / 1024) >= 10_240);
    let Some(&(name, unit)) = reached else {
        return format!("{bytes}B");
    };
    let t = tenths(unit);
    match (t / 10, t % 10) {
        (whole, 0) => format!("{whole}{name}"),
        (whole, tenth) => format!("{whole}.{tenth}{name}"),
    }
}

/// Whole milliseconds of `us`, rounded up.
pub(super) fn ms(us: u64) -> u64 {
    us.div_ceil(1000)
}
