//! `inject.*` and the `on_prompt` speed disclaimer (src/config V4).

#[cfg(test)]
mod tests;

use toml::Value;

use super::Error;
use super::parse::{BOOL, COUNT, count};
use super::source::Source;

/// Printed once when `inject.on_prompt` turns on (src/config V4).
pub const ON_PROMPT_DISCLAIMER: &str = "chert: inject.on_prompt = true: \
    every prompt now waits for a store read and a spool replay before \
    the agent sees it (cost per docs/BENCHMARKS.md; set it to false to \
    undo)";

/// `inject.*` (root C, src/hook C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inject {
    /// o200k tokens of injected context at most; default 300.
    pub max_tokens: u32,
    /// Also inject on `user-prompt-submit`; default false.
    pub on_prompt: bool,
}

impl Inject {
    /// Set `inject.on_prompt`; the speed disclaimer comes back only when
    /// this turns it on, so a caller prints it once (src/config V4).
    pub fn set_on_prompt(&mut self, on: bool) -> Option<&'static str> {
        let turned_on = on && !self.on_prompt;
        self.on_prompt = on;
        turned_on.then_some(ON_PROMPT_DISCLAIMER)
    }

    pub(super) fn build(src: &Source) -> Result<Self, Error> {
        let max = src.get("inject.max_tokens", COUNT, count)?;
        let on = src.get("inject.on_prompt", BOOL, Value::as_bool)?;
        Ok(Self {
            max_tokens: max.unwrap_or(300),
            on_prompt: on.unwrap_or(false),
        })
    }
}
