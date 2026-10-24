//! The one- and two-key sections: `search.*`, `embedding.*`,
//! `privacy.*` and `mining.*` (root C).

#[cfg(test)]
mod tests;

use toml::Value;

use super::Error;
use super::parse::{ALPHA, BOOL, PATHS, POSITIVE, alpha, positive, strings};
use super::source::Source;

/// `embedding.provider` (root C, D18).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Off,
    Ollama,
    Openai,
}

const PROVIDERS: &[(&str, Provider)] = &[
    ("off", Provider::Off),
    ("ollama", Provider::Ollama),
    ("openai", Provider::Openai),
];

/// `search.*`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Search {
    pub alpha: f64,
    pub default_limit: u32,
}

/// `embedding.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Embedding {
    pub provider: Provider,
}

/// `privacy.*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Privacy {
    pub exclude: Vec<String>,
}

/// `mining.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mining {
    pub enabled: bool,
    pub min_occurrences: u32,
}

impl Search {
    pub(super) fn build(src: &Source) -> Result<Self, Error> {
        let alpha = src.get("search.alpha", ALPHA, alpha)?;
        let limit = src.get("search.default_limit", POSITIVE, positive)?;
        Ok(Self {
            alpha: alpha.unwrap_or(0.5),
            default_limit: limit.unwrap_or(10),
        })
    }
}

impl Embedding {
    pub(super) fn build(src: &Source) -> Result<Self, Error> {
        let provider = src.pick("embedding.provider", PROVIDERS)?;
        Ok(Self {
            provider: provider.unwrap_or(Provider::Off),
        })
    }
}

impl Privacy {
    pub(super) fn build(src: &Source) -> Result<Self, Error> {
        let exclude = src.get("privacy.exclude", PATHS, strings)?;
        Ok(Self {
            exclude: exclude.unwrap_or_default(),
        })
    }
}

impl Mining {
    pub(super) fn build(src: &Source) -> Result<Self, Error> {
        let min = src.get("mining.min_occurrences", POSITIVE, positive)?;
        let enabled = src.get("mining.enabled", BOOL, Value::as_bool)?;
        Ok(Self {
            enabled: enabled.unwrap_or(true),
            min_occurrences: min.unwrap_or(3),
        })
    }
}
