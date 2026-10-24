//! Context injection (src/hook C, V4, V5; root V17).
//!
//! `session-start` with `source = startup` injects; `user-prompt-submit`
//! only with `inject.on_prompt`. The text is a header, then 1 line per
//! last 3 other sessions in this project, newest first: the ref of the
//! session's `session-end` observation and the first line of its
//! session summary; then top findings. Whole lines are cut at
//! `inject.max_tokens`, bodies never sent whole.

#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

use super::{Kind, Payload};
use crate::config::Config;
use crate::store::{Error, Filter, Ref, Scope, Store, Summary};

/// The first line of every injected context.
pub(super) const HEADER: &str = "chert: recent sessions (chert get <ref>):";

/// Other sessions injected.
const SESSIONS: usize = 3;

/// Characters of a gist at most.
const GIST: usize = 200;

/// The token budget when `kind` injects, else `None` (V4).
pub(super) fn budget(
    kind: Kind,
    payload: &Payload,
    config: &Config,
) -> Option<u32> {
    let injects = match kind {
        Kind::SessionStart => payload.source.as_deref() == Some("startup"),
        Kind::UserPromptSubmit => config.inject.on_prompt,
        _ => false,
    };
    injects.then_some(config.inject.max_tokens)
}

/// What hook `kind` prints for session `session`: its context as
/// Claude Code's `hookSpecificOutput`, or nothing when no line fits.
pub(super) fn context(
    store: &mut Store,
    kind: Kind,
    session: &str,
    max: u32,
) -> Result<Option<String>, Error> {
    let mut lines = sessions(store, session)?;
    lines.extend(findings(store));
    Ok(cut(&lines, max).map(|text| stdout(kind, &text)))
}

fn stdout(kind: Kind, text: &str) -> String {
    let out = serde_json::json!({"hookSpecificOutput": {
        "hookEventName": kind.event_name(),
        "additionalContext": text,
    }});
    out.to_string()
}

/// 1 line per last [`SESSIONS`] other sessions, newest first.
fn sessions(store: &mut Store, session: &str) -> Result<Vec<String>, Error> {
    let all = store.summaries(None, Scope::Session)?;
    let mut seen = BTreeSet::new();
    let recent = all.iter().rev().filter(|s| s.session != session);
    let fresh = recent.filter(|s| seen.insert(s.session.as_str()));
    let picked: Vec<&Summary> = fresh.take(SESSIONS).collect();
    picked.into_iter().map(|s| line(store, s)).collect()
}

fn line(store: &mut Store, summary: &Summary) -> Result<String, Error> {
    let first = summary.body.lines().next().unwrap_or_default();
    let gist: String = first.chars().take(GIST).collect();
    Ok(match end_of(store, summary)? {
        Some(reference) => format!("- {reference} {gist}"),
        None => format!("- {gist}"),
    })
}

/// The ref of the `session-end` observation written with `summary`.
fn end_of(store: &mut Store, summary: &Summary) -> Result<Option<Ref>, Error> {
    let filter = Filter {
        project: Some(store.project().to_owned()),
        since: Some(summary.ts),
        until: summary.ts.checked_add(1),
    };
    let found = store.observations(&filter)?.into_iter().find(|o| {
        o.session == summary.session && o.kind == Kind::SessionEnd.as_str()
    });
    Ok(found.map(|o| o.reference))
}

/// Top findings, 1 line each: the hook point for `src/mine`, which is
/// not built yet (root T8 order), so none.
fn findings(store: &Store) -> Vec<String> {
    let _ = store;
    Vec::new()
}

/// [`HEADER`] and the leading `lines` that fit in `max` tokens, whole
/// lines only; `None` when not 1 line fits (V5).
fn cut(lines: &[String], max: u32) -> Option<String> {
    let max = usize::try_from(max).unwrap_or(usize::MAX);
    let mut text = HEADER.to_owned();
    let mut fitted = false;
    for line in lines {
        let next = format!("{text}\n{line}");
        if tokens_at_most(&next) > max {
            break;
        }
        (text, fitted) = (next, true);
    }
    fitted.then_some(text)
}

/// The tokens `text` counts against `inject.max_tokens`, the 1 place
/// the count is chosen (src/hook C, R2): the itok estimate, bytes / 4
/// rounded up, so the hook path loads no tokenizer.
#[cfg(not(feature = "bpe"))]
fn tokens_at_most(text: &str) -> usize {
    text.len().div_ceil(4)
}

/// The exact o200k tokens of `text` (feature `bpe`); its bytes, an upper
/// bound, should the embedded table fail to load.
#[cfg(feature = "bpe")]
fn tokens_at_most(text: &str) -> usize {
    static O200K: std::sync::OnceLock<Option<tiktoken_rs::CoreBPE>> =
        std::sync::OnceLock::new();
    let bpe = O200K.get_or_init(|| tiktoken_rs::o200k_base().ok());
    bpe.as_ref()
        .map_or(text.len(), |bpe| bpe.encode_ordinary(text).len())
}
