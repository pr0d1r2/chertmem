//! The signals (src/mine G): each pattern a partition's tool calls
//! repeat, with 1 ref per occurrence, kept once it repeats at least
//! `mining.min_occurrences` times (V2).

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use super::tool::{self, Tool};
use super::{Finding, Kind};
use crate::hook::inject::tokens_at_most;
use crate::store::Observation;

/// The observation kind `src/hook` stores a tool call under.
const TOOL_KIND: &str = "post-tool-use";

/// Characters of 1 command or path a summary shows at most.
const SHOWN: usize = 80;

/// Joins the parts of a pattern; no command or path holds it.
const JOIN: char = '\0';

/// FNV-1a 64, as the store hashes (src/store R1).
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

/// A token sink holds at least 1/`SINK_SHARE` of a partition's tool call
/// tokens (src/mine R5).
const SINK_SHARE: usize = 10;

/// Every occurrence of every pattern: the row that shows it, each.
type Tally<'a> = BTreeMap<(Kind, String), Vec<&'a Observation>>;

/// The tokens of each call pattern, and the rows of its calls.
type Costs<'a> = BTreeMap<String, (usize, Vec<&'a Observation>)>;

/// 1 tool call and the row it came from.
struct Call<'a> {
    row: &'a Observation,
    tool: Tool,
}

/// The findings of `rows`, the observations of 1 partition, by count
/// (most first), then kind, then id (V1).
pub(super) fn findings(rows: &[Observation], min: u32) -> Vec<Finding> {
    let calls = calls(rows);
    let mut tally = Tally::new();
    sequences(&calls, &mut tally);
    hot_files(&calls, &mut tally);
    token_sinks(&calls, &mut tally);
    let least = usize::try_from(min).unwrap_or(usize::MAX).max(1);
    let kept = tally.into_iter().filter(|(_, seen)| seen.len() >= least);
    let mut found: Vec<Finding> = kept
        .map(|((kind, p), seen)| finding(kind, &p, seen))
        .collect();
    sort(&mut found);
    found
}

/// `found` in report order: count (most first), kind, id (V1).
pub(super) fn sort(found: &mut [Finding]) {
    found.sort_by(|a, b| {
        let key = |f: &Finding| (f.kind, f.id.clone());
        b.count.cmp(&a.count).then_with(|| key(a).cmp(&key(b)))
    });
}

/// The id of `pattern`: its FNV-1a 64, 16 hex digits (src/mine I).
pub(super) fn id(pattern: &str) -> String {
    let hash = pattern.bytes().fold(FNV_OFFSET, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(FNV_PRIME)
    });
    format!("{hash:016x}")
}

/// The tool calls of `rows`, by time then id.
fn calls(rows: &[Observation]) -> Vec<Call<'_>> {
    let mut picked: Vec<&Observation> =
        rows.iter().filter(|o| o.kind == TOOL_KIND).collect();
    picked.sort_by(|a, b| (a.ts, &a.id).cmp(&(b.ts, &b.id)));
    let parsed = picked.into_iter().map(|row| (row, tool::parse(&row.body)));
    parsed
        .filter_map(|(row, tool)| tool.map(|tool| Call { row, tool }))
        .collect()
}

/// Each Bash command right after another in 1 session, shown by the
/// row of the first.
fn sequences<'a>(calls: &[Call<'a>], tally: &mut Tally<'a>) {
    for commands in by_session(calls).values() {
        for pair in commands.windows(2) {
            if let [(a, row), (b, _)] = pair {
                let pattern = format!("{a}{JOIN}{b}");
                let seen = tally.entry((Kind::Sequence, pattern));
                seen.or_default().push(row);
            }
        }
    }
}

/// Each path a call names, shown by that call's row; a call names each
/// path once (`tool::parse`).
fn hot_files<'a>(calls: &[Call<'a>], tally: &mut Tally<'a>) {
    for call in calls {
        for path in &call.tool.paths {
            let seen = tally.entry((Kind::HotFile, path.clone()));
            seen.or_default().push(call.row);
        }
    }
}

/// Each tool call pattern whose stored bodies hold at least
/// 1/[`SINK_SHARE`] of the tokens of all `calls` (src/mine R5), shown by
/// the rows of its calls.
fn token_sinks<'a>(calls: &[Call<'a>], tally: &mut Tally<'a>) {
    let (costs, total) = costs(calls);
    for (pattern, (tokens, rows)) in costs {
        if tokens.saturating_mul(SINK_SHARE) >= total {
            tally.insert((Kind::TokenSink, pattern), rows);
        }
    }
}

/// The tokens and rows of each pattern of `calls`, and the tokens of
/// all, each body counted as `src/hook` counts its budget (src/hook R2).
fn costs<'a>(calls: &[Call<'a>]) -> (Costs<'a>, usize) {
    let (mut costs, mut total) = (Costs::new(), 0_usize);
    for call in calls {
        let tokens = tokens_at_most(&call.row.body);
        total = total.saturating_add(tokens);
        let cost = costs.entry(cost_pattern(&call.tool)).or_default();
        cost.0 = cost.0.saturating_add(tokens);
        cost.1.push(call.row);
    }
    (costs, total)
}

/// What a call's cost is counted under: its tool name, then its Bash
/// command or else the paths it names; the name alone with neither.
fn cost_pattern(tool: &Tool) -> String {
    let subject = tool.command.iter().chain(&tool.paths);
    let parts: Vec<&str> = std::iter::once(tool.name.as_str())
        .chain(subject.map(String::as_str))
        .collect();
    parts.join(&JOIN.to_string())
}

/// The Bash commands of each session, in order, with their rows.
fn by_session<'a>(
    calls: &[Call<'a>],
) -> BTreeMap<&'a str, Vec<(String, &'a Observation)>> {
    let mut found: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for call in calls {
        if let Some(command) = &call.tool.command {
            let session = found.entry(call.row.session.as_str());
            session.or_default().push((command.clone(), call.row));
        }
    }
    found
}

/// The finding of `pattern`, its refs by time then id.
fn finding(kind: Kind, pattern: &str, mut seen: Vec<&Observation>) -> Finding {
    seen.sort_by(|a, b| (a.ts, &a.id).cmp(&(b.ts, &b.id)));
    Finding {
        id: id(&format!("{}{JOIN}{pattern}", kind.as_str())),
        kind,
        count: seen.len(),
        refs: seen.into_iter().map(|o| o.reference.clone()).collect(),
        summary: summary(kind, pattern),
    }
}

/// 1 line naming `pattern`.
fn summary(kind: Kind, pattern: &str) -> String {
    let parts: Vec<String> = pattern
        .split(JOIN)
        .map(|p| format!("`{}`", shown(p)))
        .collect();
    match kind {
        Kind::Failure => String::new(),
        Kind::Sequence => parts.join(" then "),
        Kind::HotFile => format!("hot file {}", parts.join(" ")),
        Kind::TokenSink => format!(
            "token sink {}: >= 1/{SINK_SHARE} of tool call tokens",
            parts.join(" ")
        ),
    }
}

/// `text` on 1 line, cut at [`SHOWN`] characters.
fn shown(text: &str) -> String {
    let flat = |c: char| if c.is_control() { ' ' } else { c };
    let line: String = text.chars().map(flat).collect();
    if line.chars().count() <= SHOWN {
        return line;
    }
    let cut: String = line.chars().take(SHOWN).collect();
    format!("{cut}...")
}
