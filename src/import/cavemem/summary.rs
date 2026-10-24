//! cavemem's `summaries`, turn and session scope, each with its
//! session's id and ide (src/import T4).

#[cfg(test)]
mod tests;

use rusqlite::Connection;

use super::{Error, body, millis};
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
    let mut stmt = conn.prepare(SUMMARIES)?;
    let rows = stmt.query_map([], raw)?;
    let found = rows.collect::<Result<Vec<_>, _>>()?;
    found.into_iter().map(|r| summary(r, intensity)).collect()
}

/// 1 summary row as cavemem stored it.
struct Raw {
    session: String,
    scope: String,
    content: String,
    compressed: bool,
    ts: i64,
    ide: String,
}

fn raw(r: &rusqlite::Row<'_>) -> rusqlite::Result<Raw> {
    Ok(Raw {
        session: r.get(0)?,
        scope: r.get(1)?,
        content: r.get(2)?,
        compressed: r.get(3)?,
        ts: r.get(4)?,
        ide: r.get(5)?,
    })
}

fn summary(r: Raw, intensity: Intensity) -> Result<Summary, Error> {
    let scope = Scope::parse(&r.scope).ok_or_else(|| Error::Cavemem {
        reason: format!("summary scope `{}` is not turn or session", r.scope),
    })?;
    Ok(Summary {
        ts: millis(r.ts)?,
        body: body(&r.content, r.compressed, intensity),
        session: r.session,
        scope,
        ide: r.ide,
    })
}

/// Append each of `found` the store does not hold yet: a summary has no
/// identity, so each held one equal in every field answers for 1 of
/// `found`, and a summary cavemem wrote twice is kept twice (V2).
pub(super) fn put(store: &mut Store, found: &[Summary]) -> Result<(), Error> {
    let mut held = store.summaries(None, Scope::Turn)?;
    held.extend(store.summaries(None, Scope::Session)?);
    for note in found {
        if let Some(at) = held.iter().position(|h| h == note) {
            held.swap_remove(at);
        } else {
            store.append_summary(note)?;
        }
    }
    Ok(())
}
