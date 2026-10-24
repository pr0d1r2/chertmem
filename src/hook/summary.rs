//! Turn and session summaries, as cavemem 0.2.1 writes them, no model
//! (src/hook C, V6).
//!
//! A turn summary is the `stop` event's body: `last_assistant_message`,
//! redacted and compressed. A session summary is its first [`TURNS`]
//! turn summaries, 1 per line, so its first line is the first turn's.

#[cfg(test)]
mod tests;

use super::Kind;
use crate::store::{Error, Event, Scope, Store, Summary};

/// The turns a session summary rolls up.
pub(super) const TURNS: usize = 20;

/// What follows `kind`'s stored `event`: a turn summary on `stop`, a
/// session roll-up on `session-end`; nothing for an empty body.
pub(super) fn after(
    store: &mut Store,
    kind: Kind,
    event: &Event,
) -> Result<(), Error> {
    let (scope, body) = match kind {
        Kind::Stop => (Scope::Turn, event.body.clone()),
        Kind::SessionEnd => {
            let turns = store.summaries(Some(&event.session), Scope::Turn)?;
            (Scope::Session, rollup(&turns))
        }
        _ => return Ok(()),
    };
    if !body.is_empty() {
        store.append_summary(&note(event, scope, body))?;
    }
    Ok(())
}

fn note(event: &Event, scope: Scope, body: String) -> Summary {
    Summary {
        ts: event.ts,
        session: event.session.clone(),
        scope,
        ide: event.ide.clone(),
        body,
    }
}

/// The first [`TURNS`] turn bodies, 1 per line.
fn rollup(turns: &[Summary]) -> String {
    let bodies: Vec<&str> =
        turns.iter().take(TURNS).map(|t| t.body.as_str()).collect();
    bodies.join("\n")
}
