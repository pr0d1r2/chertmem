//! A session's row: its `cwd` and start on `session-start`, its end on
//! `session-end`, through the store (src/hook T10, src/store T16).
//!
//! Both calls work from a write-only open and spool under a held lock
//! (src/store V10), so the write-only path stays within the budget.

#[cfg(test)]
mod tests;

use super::Kind;
use crate::store::{Error, Event, SessionStart, Store};

/// Record the session row `kind`'s stored `event` changes, if any;
/// `cwd` is the working dir the payload reports.
pub(super) fn record(
    store: &mut Store,
    kind: Kind,
    event: &Event,
    cwd: Option<&str>,
) -> Result<(), Error> {
    let written = match kind {
        Kind::SessionStart => store.session_started(&start(event, cwd)),
        Kind::SessionEnd => store.session_ended(&event.session, event.ts),
        _ => return Ok(()),
    };
    written.map(drop)
}

/// The start `event` reports; no `cwd` reported records an empty one.
fn start(event: &Event, cwd: Option<&str>) -> SessionStart {
    SessionStart {
        id: event.session.clone(),
        ide: event.ide.clone(),
        cwd: cwd.unwrap_or_default().to_owned(),
        ts: event.ts,
    }
}
