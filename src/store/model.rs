//! The values the store takes and gives back (src/store I).

#[cfg(test)]
mod tests;

use super::Ref;

/// 1 captured event, its body already compressed by the caller
/// (`src/hook` runs `chertmem::grammar` first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Milliseconds since the Unix epoch: the event's own time, which
    /// picks its bucket (src/store V1).
    pub ts: u64,
    pub session: String,
    pub kind: String,
    pub ide: String,
    /// The identity the IDE dialect supplies (a tool-use id); `None` →
    /// a new row every time (root V16).
    pub identity: Option<String>,
    pub body: String,
}

/// What became of an append (src/store V9, V10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Appended {
    /// In its partition, under this ref; for a known identity the ref
    /// of the first append.
    Stored(Ref),
    /// A lock outlasted the busy timeout: the event waits in `spool/`
    /// for the next reading open, which issues its ref.
    Spooled,
}

/// A partition's tier (src/store C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Hot,
    Warm,
    Cold,
}

const STATES: &[(&str, State)] = &[
    ("hot", State::Hot),
    ("warm", State::Warm),
    ("cold", State::Cold),
];

impl State {
    /// The catalog's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        STATES
            .iter()
            .find(|&&(_, s)| s == self)
            .map_or("hot", |&(name, _)| name)
    }

    /// `hot`, `warm` or `cold`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        STATES.iter().find(|(n, _)| *n == text).map(|&(_, s)| s)
    }
}
