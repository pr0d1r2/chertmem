//! A session's row: its `cwd` and start on `session-start`, its end on
//! `session-end`, through the store (src/hook T10, src/store T16).
//!
//! Both calls work from a write-only open and spool under a held lock
//! (src/store V10), so the write-only path stays within the budget.

#[cfg(test)]
mod tests;

use super::Kind;
use crate::store::{Error, Event, Store};

/// Record the session row `kind`'s stored `event` changes, if any;
/// `cwd` is the working dir the payload reports.
pub(super) fn record(
    store: &mut Store,
    kind: Kind,
    event: &Event,
    _cwd: Option<&str>,
) -> Result<(), Error> {
    if kind == Kind::SessionEnd {
        store.session_ended(&event.session, event.ts)?;
    }
    Ok(())
}
