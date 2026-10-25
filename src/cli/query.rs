//! `chert search`, `chert timeline` and `chert get` (root I): the MCP
//! tools' answers on the command line, 1 row per line.

#[cfg(test)]
mod tests;

use clap::{Arg, ArgAction, ArgMatches, value_parser};

use super::args::{Since, flag, many, one, option, since_arg, value, verb};
use super::{Done, Io, fail, load, open, say};
use crate::search::{self, Engine, Query};
use crate::store::{Lookup, Observation, Ref};

#[derive(Debug)]
pub(super) enum Verb {
    Search(Search),
    Timeline {
        session: String,
        around: Option<String>,
    },
    Get {
        refs: Vec<String>,
    },
}

#[derive(Debug)]
pub(super) struct Search {
    query: Vec<String>,
    limit: Option<u32>,
    since: Option<Since>,
    project: Option<String>,
    no_semantic: bool,
}

impl Search {
    fn parse(m: &ArgMatches) -> Self {
        Self {
            query: many(m, "query"),
            limit: one(m, "limit"),
            since: one(m, "since"),
            project: one(m, "project"),
            no_semantic: m.get_flag("no_semantic"),
        }
    }
}

/// `search`, `timeline` and `get`.
pub(super) fn commands() -> Vec<clap::Command> {
    let timeline = verb("timeline", "A session's rows in order, around 1 ref")
        .arg(
            value("session", "SESSION", value_parser!(String))
                .help("The IDE's session id"),
        )
        .arg(
            option("around", "AROUND", value_parser!(String))
                .help("Centre on this `chert:<ref>`"),
        );
    let get = verb(
        "get",
        "Observations by `chert:<ref>`; exit 1 when one is dropped",
    )
    .arg(words("refs", "REFS"));
    vec![search_command(), timeline, get]
}

fn search_command() -> clap::Command {
    let query = words("query", "QUERY")
        .help("The query; several words are joined by spaces");
    let limit = option("limit", "LIMIT", value_parser!(u32))
        .help("Hits at most; default `search.default_limit`");
    let project = option("project", "PROJECT", value_parser!(String))
        .help("Only this project id, as `chert partitions list` prints it");
    let no_semantic = flag("no_semantic", "NO_SEMANTIC", "no-semantic")
        .help("Keyword ranking only, even when embeddings are on");
    verb("search", "Ranked search across partitions, best first").args([
        query,
        limit,
        since_arg(),
        project,
        no_semantic,
    ])
}

/// `<NAME>...`: 1 or more words, required.
fn words(id: &'static str, name: &'static str) -> Arg {
    let arg = Arg::new(id).value_name(name).num_args(1..);
    let arg = arg.value_parser(value_parser!(String));
    arg.action(ArgAction::Append).required(true)
}

/// The verb `name` names, when it is 1 of this group's.
pub(super) fn parse(name: &str, m: &ArgMatches) -> Option<Verb> {
    match name {
        "search" => Some(Verb::Search(Search::parse(m))),
        "timeline" => Some(Verb::Timeline {
            session: one(m, "session")?,
            around: one(m, "around"),
        }),
        "get" => Some(Verb::Get {
            refs: many(m, "refs"),
        }),
        _ => None,
    }
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
