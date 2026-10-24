//! Sizes, durations and the other value forms `chert.toml` accepts
//! (src/config C, V5).

#[cfg(test)]
mod tests;

use std::fmt;

use toml::Value;

pub(super) const SPAN_FORMS: &str = "<n>d, <n>h or <n>mo with n >= 1";
pub(super) const SIZE_FORMS: &str = "<n>KiB, <n>MiB or <n>GiB with n >= 1";
pub(super) const POSITIVE: &str = "an integer >= 1";
pub(super) const COUNT: &str = "an integer >= 0";
pub(super) const BOOL: &str = "true or false";
pub(super) const ALPHA: &str = "a number from 0 to 1";
pub(super) const PATH: &str = "a path string";
pub(super) const PATHS: &str = "an array of path strings";

/// The unit of a [`Span`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Hours,
    Days,
    Months,
}

/// A duration as written: `<n>d`, `<n>h` or `<n>mo` (src/config C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub amount: u32,
    pub unit: Unit,
}

const SPAN_UNITS: &[(&str, Unit)] =
    &[("mo", Unit::Months), ("d", Unit::Days), ("h", Unit::Hours)];

impl Span {
    /// Parse `<n>d`, `<n>h` or `<n>mo`, n >= 1.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let (amount, unit) = split_unit(text, SPAN_UNITS)?;
        Some(Self { amount, unit })
    }

    /// The span in hours; `None` for months.
    pub(super) fn hours(self) -> Option<u64> {
        let amount = u64::from(self.amount);
        match self.unit {
            Unit::Hours => Some(amount),
            Unit::Days => amount.checked_mul(24),
            Unit::Months => None,
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suffix = SPAN_UNITS
            .iter()
            .find(|&&(_, unit)| unit == self.unit)
            .map_or("", |&(suffix, _)| suffix);
        write!(f, "{}{suffix}", self.amount)
    }
}

const SIZE_UNITS: &[(&str, u64)] =
    &[("KiB", 1 << 10), ("MiB", 1 << 20), ("GiB", 1 << 30)];

/// Parse a size `<n>KiB`, `<n>MiB` or `<n>GiB`, n >= 1, into bytes.
#[must_use]
pub fn parse_size(text: &str) -> Option<u64> {
    let (amount, scale) = split_unit(text, SIZE_UNITS)?;
    u64::from(amount).checked_mul(scale)
}

/// `<digits><suffix>` for the first suffix that matches, digits >= 1.
fn split_unit<T: Copy>(text: &str, units: &[(&str, T)]) -> Option<(u32, T)> {
    units.iter().find_map(|&(suffix, unit)| {
        let digits = text.strip_suffix(suffix)?;
        Some((positive_digits(digits)?, unit))
    })
}

fn positive_digits(digits: &str) -> Option<u32> {
    if !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u32>().ok().filter(|n| *n > 0)
}

pub(super) fn name<T: Copy>(value: &Value, names: &[(&str, T)]) -> Option<T> {
    let text = value.as_str()?;
    names.iter().find(|(n, _)| *n == text).map(|&(_, t)| t)
}

pub(super) fn span(value: &Value) -> Option<Span> {
    value.as_str().and_then(Span::parse)
}

pub(super) fn size(value: &Value) -> Option<u64> {
    value.as_str().and_then(parse_size)
}

pub(super) fn count(value: &Value) -> Option<u32> {
    u32::try_from(value.as_integer()?).ok()
}

pub(super) fn positive(value: &Value) -> Option<u32> {
    count(value).filter(|n| *n > 0)
}

pub(super) fn alpha(value: &Value) -> Option<f64> {
    let int = || value.as_integer().and_then(|n| i32::try_from(n).ok());
    let a = value.as_float().or_else(|| int().map(f64::from))?;
    (0.0..=1.0).contains(&a).then_some(a)
}

pub(super) fn strings(value: &Value) -> Option<Vec<String>> {
    let items = value.as_array()?.iter();
    items.map(|v| v.as_str().map(str::to_owned)).collect()
}
