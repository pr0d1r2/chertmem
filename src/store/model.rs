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

/// Which partitions and rows a read wants; `None` = unbounded. Time is
/// `[since, until)` in milliseconds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    /// A project id ([`super::project_id`]).
    pub project: Option<String>,
    pub since: Option<u64>,
    pub until: Option<u64>,
}

/// A partition's size and content hash (src/store C, R1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stats {
    pub rows: u64,
    /// The bytes of every observation body.
    pub bytes: u64,
    /// FNV-1a 64 over (id, body) of every row in insert order, hex.
    pub checksum: String,
}

/// A partition's catalog row (src/store C).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    /// Relative to the data dir.
    pub path: String,
    /// `partition.by` it was written under, e.g. `project,time`.
    pub dims: String,
    /// The project dim; empty when `project` was not a dim.
    pub project: String,
    /// The ide dim; empty when `ide` was not a dim.
    pub ide: String,
    pub t0: u64,
    pub t1: u64,
    pub shard: Option<u32>,
    pub stats: Stats,
    pub state: State,
    /// The `partition.*` spec it was written under.
    pub spec: String,
}

/// What a ref resolves to (root V5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lookup {
    Found(Observation),
    /// Gone: never another row in its place.
    Dropped(Ref),
}

/// 1 stored observation (src/store C).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub reference: Ref,
    /// The ULID.
    pub id: String,
    pub project: String,
    pub ts: u64,
    pub session: String,
    pub ide: String,
    pub kind: String,
    pub identity: Option<String>,
    /// As appended: compressed by the caller.
    pub body: String,
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
