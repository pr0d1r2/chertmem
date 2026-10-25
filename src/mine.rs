//! Behaviour findings with evidence refs (src/mine G): repeated failing
//! commands and errors, command sequences, hot files and token sinks,
//! mined from the `post-tool-use` and `post-tool-use-failure`
//! observations `src/hook` stores. Mining only reads the store; it changes nothing
//! by itself (root C) and never writes under `.rekall/` (V3).

#[cfg(test)]
mod tests;

mod error;
mod ledger;
mod report;
mod run;
mod signal;
mod tool;

pub use error::Error;
pub use report::{Format, report};
pub use run::mine;

use crate::store::{Observation, Ref};

/// The findings of `rows`, the observations of 1 partition or of any
/// span read at once (a hot partition is mined on demand, src/mine C):
/// every pattern repeated at least `min` times, by count (most first),
/// then kind, then id (V1, V2).
#[must_use]
pub fn findings(rows: &[Observation], min: u32) -> Vec<Finding> {
    signal::findings(rows, min)
}

/// What a finding is a repeat of (src/mine G).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// 1 tool call pattern failing with 1 error, from the
    /// `post-tool-use-failure` observations (src/mine T1).
    Failure,
    /// 1 Bash command right after another in 1 session.
    Sequence,
    /// 1 path a tool read or wrote.
    HotFile,
    /// 1 tool call pattern whose stored bodies cost a large share of the
    /// tokens of every tool call (src/mine R5).
    TokenSink,
}

const KINDS: [(Kind, &str); 4] = [
    (Kind::Failure, "failure"),
    (Kind::Sequence, "sequence"),
    (Kind::HotFile, "hot_file"),
    (Kind::TokenSink, "token_sink"),
];

impl Kind {
    /// The spelling in a report and in the finding store.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        KINDS
            .iter()
            .find(|&&(k, _)| k == self)
            .map_or("sequence", |&(_, name)| name)
    }

    /// `failure`, `sequence`, `hot_file` or `token_sink`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        KINDS.iter().find(|(_, n)| *n == text).map(|&(k, _)| k)
    }
}

/// 1 behaviour finding (src/mine I): `id` is the hash of its pattern, so
/// the same pattern has the same id in every run (V1); `count` is the
/// number of `refs`, 1 per occurrence (V2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub id: String,
    pub kind: Kind,
    pub count: usize,
    pub refs: Vec<Ref>,
    pub summary: String,
}
