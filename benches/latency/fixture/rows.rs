//! The rows of a fixture, generated from its seed alone: sessions of 10
//! to 79 events, each opened by `session-start` and closed by
//! `session-end` with a session summary, the rest tool calls, prompts
//! and stops as a Claude Code session has them (benches T2, V2).

#[cfg(test)]
mod tests;

use std::collections::VecDeque;

use chertmem::hook::Kind;
use chertmem::store::{Event, Scope, SessionStart, Summary};

use super::Spec;
use crate::rng::Rng;
use crate::words::{any, path, prose};

/// The IDE every fixture row comes from.
pub(crate) const IDE: &str = "claude-code";

/// 1 generated write, in the order the hooks would make it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Row {
    Begin(SessionStart),
    Event(Event),
    /// A session's summary, written with its `session-end` event.
    Summary(Summary),
}

/// The rows of a [`Spec`]: exactly `size` events, by time.
#[derive(Debug, Clone)]
pub(crate) struct Rows {
    rng: Rng,
    spec: Spec,
    /// Events generated so far.
    done: u64,
    sessions: u64,
    queue: VecDeque<Row>,
}

impl Rows {
    pub(crate) fn new(spec: Spec) -> Self {
        Self {
            rng: Rng::new(spec.seed),
            spec,
            done: 0,
            sessions: 0,
            queue: VecDeque::new(),
        }
    }

    /// Queue the next session's rows.
    fn session(&mut self) {
        let left = self.spec.size.saturating_sub(self.done);
        let len = self.rng.below(70).saturating_add(10).min(left);
        let id = format!("s{}-{}", self.spec.seed, self.sessions);
        self.sessions = self.sessions.saturating_add(1);
        for k in 0..len {
            let last = k.saturating_add(1) == len && len > 1;
            self.step(&id, k == 0, last);
        }
    }

    /// Queue 1 event of session `id`, with what it writes beside it.
    fn step(&mut self, id: &str, first: bool, last: bool) {
        let ts = self.spec.ts(self.done);
        let kind = self.kind(first, last);
        if first {
            self.queue.push_back(Row::Begin(begin(id, ts)));
        }
        let event = self.event(kind, id, ts);
        self.queue.push_back(Row::Event(event));
        if kind == Kind::SessionEnd {
            let body = summary(&mut self.rng);
            self.queue.push_back(Row::Summary(close(id, ts, body)));
        }
        self.done = self.done.saturating_add(1);
    }

    /// The kind of an event: fixed at either end, else drawn.
    fn kind(&mut self, first: bool, last: bool) -> Kind {
        match (first, last, self.rng.below(100)) {
            (true, _, _) => Kind::SessionStart,
            (_, true, _) => Kind::SessionEnd,
            (_, _, 0..=69) => Kind::PostToolUse,
            (_, _, 70..=74) => Kind::PostToolUseFailure,
            (_, _, 75..=89) => Kind::UserPromptSubmit,
            _ => Kind::Stop,
        }
    }

    fn event(&mut self, kind: Kind, session: &str, ts: u64) -> Event {
        let calls =
            matches!(kind, Kind::PostToolUse | Kind::PostToolUseFailure);
        let tool_id = format!("toolu_{}_{}", self.spec.seed, self.done);
        Event {
            ts,
            session: session.to_owned(),
            kind: kind.as_str().to_owned(),
            ide: IDE.to_owned(),
            identity: calls.then_some(tool_id),
            body: body(&mut self.rng, kind),
        }
    }
}

impl Iterator for Rows {
    type Item = Row;

    fn next(&mut self) -> Option<Row> {
        if self.queue.is_empty() && self.done < self.spec.size {
            self.session();
        }
        self.queue.pop_front()
    }
}

fn begin(id: &str, ts: u64) -> SessionStart {
    SessionStart {
        id: id.to_owned(),
        ide: IDE.to_owned(),
        cwd: "/work/project".to_owned(),
        ts,
    }
}

fn close(id: &str, ts: u64, body: String) -> Summary {
    Summary {
        ts,
        session: id.to_owned(),
        scope: Scope::Session,
        ide: IDE.to_owned(),
        body,
    }
}

/// The text a hook stores for `kind`, before compression.
fn body(rng: &mut Rng, kind: Kind) -> String {
    match kind {
        Kind::SessionStart => "startup".to_owned(),
        Kind::SessionEnd => "prompt_input_exit".to_owned(),
        Kind::UserPromptSubmit => prose(rng, 6, 20),
        Kind::Stop => prose(rng, 10, 30),
        Kind::PostToolUse => tool(rng),
        Kind::PostToolUseFailure => failure(rng),
    }
}

/// Tool name, input as JSON, response as JSON (`src/hook` T9).
fn tool(rng: &mut Rng) -> String {
    let (name, input) = call(rng);
    let out = prose(rng, 5, 40);
    format!("{name}\n{input}\n{{\"stdout\":\"{out}\"}}")
}

/// A failed call: its error text on line 3 (`src/hook` T9).
fn failure(rng: &mut Rng) -> String {
    let (name, input) = call(rng);
    let error = prose(rng, 4, 8);
    format!("{name}\n{input}\nerror: {error}")
}

/// A tool and its input: shell, read, edit or grep.
fn call(rng: &mut Rng) -> (&'static str, String) {
    let (w, p) = (any(rng), path(rng));
    match rng.below(10) {
        0..=3 => ("Bash", format!("{{\"command\":\"cargo test -p {w}\"}}")),
        4..=6 => ("Read", format!("{{\"file_path\":\"{p}\"}}")),
        7 | 8 => ("Edit", edit(rng, &p)),
        _ => ("Grep", format!("{{\"path\":\"src\",\"pattern\":\"{w}\"}}")),
    }
}

fn edit(rng: &mut Rng, path: &str) -> String {
    let (old, new) = (prose(rng, 2, 5), prose(rng, 2, 5));
    format!(
        "{{\"file_path\":\"{path}\",\"new_string\":\"{new}\",\
         \"old_string\":\"{old}\"}}"
    )
}

/// A session summary: a path worked on, then what was done.
fn summary(rng: &mut Rng) -> String {
    let (p, done) = (path(rng), prose(rng, 8, 12));
    format!("worked on {p}: {done}")
}
