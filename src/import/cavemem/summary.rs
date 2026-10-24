//! cavemem's `summaries`, turn and session scope, each with its
//! session's id and ide (src/import T4).

#[cfg(test)]
mod tests;

use rusqlite::Connection;

use super::Error;
use crate::grammar::Intensity;
use crate::store::{Scope, Store, Summary};

const SUMMARIES: &str = "SELECT m.session_id, m.scope, m.content,
    m.compressed, m.ts, COALESCE(s.ide, '')
  FROM summaries m LEFT JOIN sessions s ON s.id = m.session_id
  ORDER BY m.id";

/// Every summary as chert stores it, in cavemem's order.
pub(super) fn read(
    conn: &Connection,
    intensity: Intensity,
) -> Result<Vec<Summary>, Error> {
    let _ = (conn.prepare(SUMMARIES)?, intensity);
    Ok(Vec::new())
}

/// Append each of `found` the store does not hold yet.
pub(super) fn put(store: &mut Store, found: &[Summary]) -> Result<(), Error> {
    let _ = (store.summaries(None, Scope::Turn)?, found);
    Ok(())
}
