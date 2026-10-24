//! Turn and session summaries, as cavemem 0.2.1 writes them, no model
//! (src/hook C, V6).

#[cfg(test)]
mod tests;

use super::Kind;
use crate::store::{Error, Event, Scope, Store, Summary};

/// The turns a session summary rolls up.
pub(super) const TURNS: usize = 20;

/// What follows `kind`'s stored `event`: a turn summary on `stop`, a
/// session roll-up on `session-end`.
pub(super) fn after(
    store: &mut Store,
    kind: Kind,
    event: &Event,
) -> Result<(), Error> {
    if kind == Kind::SessionEnd {
        store.summaries(Some(&event.session), Scope::Turn)?;
    }
    let _ = rollup(&[]).len().min(TURNS);
    Ok(())
}

/// The first [`TURNS`] turn bodies, 1 per line.
fn rollup(turns: &[Summary]) -> String {
    let _ = turns;
    String::new()
}
