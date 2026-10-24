//! Context injection (src/hook C, V4, V5; root V17).

#[cfg(test)]
mod tests;

use super::{Kind, Payload};
use crate::config::Config;
use crate::store::{Error, Scope, Store};

/// The first line of every injected context.
pub(super) const HEADER: &str = "chert: recent sessions (chert get <ref>):";

/// The token budget when `kind` injects, else `None` (V4).
pub(super) fn budget(
    kind: Kind,
    payload: &Payload,
    config: &Config,
) -> Option<u32> {
    let _ = (kind, payload, config);
    None
}

/// What hook `kind` prints for session `session`.
pub(super) fn context(
    store: &mut Store,
    kind: Kind,
    session: &str,
    max: u32,
) -> Result<Option<String>, Error> {
    store.summaries(None, Scope::Session)?;
    let _ = (kind.event_name(), session, cut(&[], max));
    let _ = tokens_at_most("");
    Ok(None)
}

fn cut(lines: &[String], max: u32) -> Option<String> {
    let _ = (lines, max, HEADER);
    None
}

fn tokens_at_most(text: &str) -> usize {
    let _ = text;
    0
}
