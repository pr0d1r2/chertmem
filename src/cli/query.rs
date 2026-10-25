//! `chert search`, `chert timeline` and `chert get` (root I): the MCP
//! tools' answers on the command line, 1 row per line.

#[cfg(test)]
mod tests;

use clap::{Args, Subcommand};

use super::args::{Since, since};
use super::{Done, Io, fail, load, open, say};
use crate::search::{self, Engine, Query};
use crate::store::{Lookup, Observation, Ref};

#[derive(Debug, Subcommand)]
pub(super) enum Verb {
    /// Ranked search across partitions, best first
    Search(Search),
    /// A session's rows in order, around 1 ref
    Timeline {
        /// The IDE's session id
        session: String,
        /// Centre on this `chert:<ref>`
        #[arg(long)]
        around: Option<String>,
    },
    /// Observations by `chert:<ref>`; exit 1 when one is dropped
    Get {
        #[arg(required = true)]
        refs: Vec<String>,
    },
}

#[derive(Debug, Args)]
pub(super) struct Search {
    /// The query; several words are joined by spaces
    #[arg(required = true)]
    query: Vec<String>,
    /// Hits at most; default `search.default_limit`
    #[arg(long)]
    limit: Option<u32>,
    /// Only rows since `<n>h`, `<n>d`, `<n>mo` ago, or ms since epoch
    #[arg(long, value_parser = since)]
    since: Option<Since>,
    /// Only this project id, as `chert partitions list` prints it
    #[arg(long)]
    project: Option<String>,
    /// Keyword ranking only, even when embeddings are on
    #[arg(long)]
    no_semantic: bool,
}

pub(super) fn run(verb: Verb, io: &mut Io<'_>) -> Done {
    match verb {
        Verb::Search(args) => search(args, io),
        Verb::Timeline { session, around } => {
            timeline(&session, around.as_deref(), io)
        }
        Verb::Get { refs } => get(&refs, io),
    }
}

/// `<ref> <score> <session> <ts> <snippet>` per hit; none is exit 0.
fn search(args: Search, io: &mut Io<'_>) -> Done {
    let mut engine = Engine::open(&load(io)?, io.cwd).map_err(fail)?;
    let query = Query {
        text: args.query.join(" "),
        limit: args.limit,
        since: args.since.map(|s| s.ms((io.now)())),
        project: args.project,
        semantic: !args.no_semantic,
        ..Query::default()
    };
    for h in engine.run(&query, None).map_err(fail)? {
        let line =
            format!("{} {:.4} {} {}", h.reference, h.score, h.session, h.ts);
        say(io, &format!("{line} {}", h.snippet.replace('\n', " ")))?;
    }
    Ok(0)
}

/// `<ref> <kind> <ts>` per row; `--around` a dropped ref is exit 1.
fn timeline(session: &str, around: Option<&str>, io: &mut Io<'_>) -> Done {
    let around = around.map(Ref::parse).transpose().map_err(fail)?;
    let mut engine = Engine::open(&load(io)?, io.cwd).map_err(fail)?;
    match engine.timeline(session, around.as_ref(), None) {
        Ok(rows) => {
            for e in rows {
                say(io, &format!("{} {} {}", e.reference, e.kind, e.ts))?;
            }
            Ok(0)
        }
        Err(e @ search::Error::Dropped { .. }) => dropped(io, &e),
        Err(e) => Err(fail(e)),
    }
}

/// `why` on stderr: exit 1, a finding.
fn dropped(io: &mut Io<'_>, why: &search::Error) -> Done {
    writeln!(io.stderr, "{why}").map_err(fail)?;
    Ok(1)
}

/// Each ref's row, or `<ref> dropped`; any dropped is exit 1.
fn get(texts: &[String], io: &mut Io<'_>) -> Done {
    let parsed: Result<Vec<Ref>, _> =
        texts.iter().map(|t| Ref::parse(t)).collect();
    let refs = parsed.map_err(fail)?;
    let found = open(&load(io)?, io)?.get(&refs).map_err(fail)?;
    let mut exit = 0;
    for lookup in found {
        match lookup {
            Lookup::Found(row) => say(io, &shown(&row))?,
            Lookup::Dropped(r) => {
                say(io, &format!("{r} dropped"))?;
                exit = 1;
            }
        }
    }
    Ok(exit)
}

/// `<ref> <kind> <ts> <session>`, then the body as stored.
fn shown(row: &Observation) -> String {
    let Observation {
        reference,
        kind,
        ts,
        session,
        body,
        ..
    } = row;
    format!("{reference} {kind} {ts} {session}\n{body}")
}
