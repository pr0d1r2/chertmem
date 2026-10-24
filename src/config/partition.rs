//! `partition.*` (src/config C, root C D19).

#[cfg(test)]
mod tests;

use toml::Value;

use super::Error;
use super::parse::{POSITIVE, SPAN_FORMS, Span, Unit, name, positive, span};
use super::source::Source;

const BY_FORMS: &str =
    "an array of distinct \"project\", \"time\", \"ide\" holding \"time\"";

/// A `partition.by` dimension (src/config C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dimension {
    Project,
    Time,
    Ide,
}

const DIMENSIONS: &[(&str, Dimension)] = &[
    ("project", Dimension::Project),
    ("time", Dimension::Time),
    ("ide", Dimension::Ide),
];

/// `partition.*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    pub by: Vec<Dimension>,
    pub time_bucket: Span,
    pub shards: u32,
}

/// Root C D19 default.
const TIME_BUCKET: Span = Span {
    amount: 7,
    unit: Unit::Days,
};

impl Partition {
    pub(super) fn build(src: &Source) -> Result<Self, Error> {
        let by = src.get("partition.by", BY_FORMS, dimensions)?;
        let bucket = src.get("partition.time_bucket", SPAN_FORMS, span)?;
        let shards = src.get("partition.shards", POSITIVE, positive)?;
        Ok(Self {
            by: by.unwrap_or_else(|| vec![Dimension::Project, Dimension::Time]),
            time_bucket: bucket.unwrap_or(TIME_BUCKET),
            shards: shards.unwrap_or(1),
        })
    }
}

/// Distinct dimensions, in order, holding `time` (src/config C).
fn dimensions(value: &Value) -> Option<Vec<Dimension>> {
    let items = value.as_array()?.iter();
    let dims: Vec<Dimension> =
        items.map(|v| name(v, DIMENSIONS)).collect::<Option<_>>()?;
    let mut seen = Vec::new();
    let distinct = dims.iter().all(|d| {
        let new = !seen.contains(d);
        seen.push(*d);
        new
    });
    (distinct && dims.contains(&Dimension::Time)).then_some(dims)
}
