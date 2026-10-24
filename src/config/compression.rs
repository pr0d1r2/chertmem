//! `compression.*`: the caveman grammar and the `compression.zstd`
//! table (root C).

#[cfg(test)]
mod tests;

use toml::Value;

use super::Error;
use super::parse::BOOL;
use super::source::Source;

const ZSTD_FORMS: &str = "a table { level = <1 to 22>, dictionary = <true \
    or false> }, each key optional";
const LEVEL: &str = "an integer from 1 to 22";

/// `compression.grammar`, the caveman intensity (src/grammar G).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grammar {
    Off,
    Lite,
    Full,
    Ultra,
}

const GRAMMARS: &[(&str, Grammar)] = &[
    ("off", Grammar::Off),
    ("lite", Grammar::Lite),
    ("full", Grammar::Full),
    ("ultra", Grammar::Ultra),
];

/// `compression.zstd`, a table (root C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zstd {
    /// zstd level, 1 to 22; default 3.
    pub level: i32,
    /// Train and use a dictionary per partition; default true.
    pub dictionary: bool,
}

/// `compression.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Compression {
    pub grammar: Grammar,
    pub zstd: Zstd,
}

impl Compression {
    pub(super) fn build(src: &Source) -> Result<Self, Error> {
        let grammar = src.pick("compression.grammar", GRAMMARS)?;
        Ok(Self {
            grammar: grammar.unwrap_or(Grammar::Full),
            zstd: Zstd::build(src)?,
        })
    }
}

impl Zstd {
    /// A `compression.zstd` that is not a table is invalid; inside the
    /// table each key defaults on its own (V1, V3).
    fn build(src: &Source) -> Result<Self, Error> {
        src.get("compression.zstd", ZSTD_FORMS, |_| None::<()>)?;
        let level = src.get("compression.zstd.level", LEVEL, level)?;
        let dict = src.get("compression.zstd.dictionary", BOOL, Value::as_bool);
        Ok(Self {
            level: level.unwrap_or(3),
            dictionary: dict?.unwrap_or(true),
        })
    }
}

fn level(value: &Value) -> Option<i32> {
    let n = i32::try_from(value.as_integer()?).ok()?;
    (1..=22).contains(&n).then_some(n)
}
