//! 1 append: identity check, ref issue, partition insert, then the
//! catalog, all under the catalog's write lock (src/store V6, V9).

#[cfg(test)]
mod tests;

use rusqlite::{Connection, Transaction, TransactionBehavior};

use super::catalog::{self, Slot};
use super::layout::{Key, Plan};
use super::partition::{self, Chunks};
use super::reference;
use super::session::{self, Begun, Ended};
use super::summary;
use super::{Error, Event, Ref, Summary};

/// An event with the id and project it is stored under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Row {
    pub id: String,
    pub project: String,
    pub event: Event,
}

impl Row {
    pub(super) fn key(&self) -> Key<'_> {
        Key {
            project: &self.project,
            ide: &self.event.ide,
            ts: self.event.ts,
            id: &self.id,
        }
    }
}

/// A summary with the id and project it is stored under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Note {
    pub id: String,
    pub project: String,
    pub summary: Summary,
}

impl Note {
    pub(super) fn key(&self) -> Key<'_> {
        Key {
            project: &self.project,
            ide: &self.summary.ide,
            ts: self.summary.ts,
            id: &self.id,
        }
    }
}

/// Anything the spool holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Item {
    Observation(Row),
    Summary(Note),
    Begun(Begun),
    Ended(Ended),
}

/// Store a replayed item, whichever it is.
pub(super) fn apply(cx: &mut Ctx<'_>, item: &Item) -> Result<(), Error> {
    match item {
        Item::Observation(row) => observation(cx, row).map(drop),
        Item::Summary(note) => summary(cx, note),
        Item::Begun(begun) => session::start(cx.catalog, begun),
        Item::Ended(ended) => session::end(cx.catalog, ended),
    }
}

/// Store `note` in the hot partition of its own timestamp; its id seen
/// before stores nothing (a replayed spool file).
pub(super) fn summary(cx: &mut Ctx<'_>, note: &Note) -> Result<(), Error> {
    let tx = immediate(cx.catalog)?;
    let (_, conn) = slot(cx.plan, cx.chunks, &tx, note.key())?;
    summary::insert(conn, note)?;
    Ok(tx.commit()?)
}

/// What a write reaches.
#[derive(Debug)]
pub(super) struct Ctx<'a> {
    pub catalog: &'a Connection,
    pub chunks: &'a mut Chunks,
    pub plan: &'a Plan,
}

/// The catalog's write lock: 1 writer across processes, the busy timeout
/// then [`Error::Busy`].
pub(super) fn immediate(conn: &Connection) -> Result<Transaction<'_>, Error> {
    let behavior = TransactionBehavior::Immediate;
    Ok(Transaction::new_unchecked(conn, behavior)?)
}

/// The hot partition `key` lands in: its catalog row, then its file
/// (src/store V7).
pub(super) fn slot<'c>(
    plan: &Plan,
    chunks: &'c mut Chunks,
    tx: &Connection,
    key: Key<'_>,
) -> Result<(Slot, &'c Connection), Error> {
    let place = plan.place(key)?;
    let (dims, spec) = (plan.dims(), plan.spec());
    let slot = catalog::ensure(tx, &place, &dims, &spec)?;
    Ok((slot, chunks.hot(&place, &dims, &spec)?))
}

/// Store `row`; its ref, the first one for a known id or identity.
pub(super) fn observation(cx: &mut Ctx<'_>, row: &Row) -> Result<Ref, Error> {
    let tx = immediate(cx.catalog)?;
    if let Some(found) = catalog::existing(&tx, row)? {
        return Ok(Ref::stored(found));
    }
    let (slot, conn) = slot(cx.plan, cx.chunks, &tx, row.key())?;
    let issued = issue(&tx, &row.project, &row.id)?;
    let stored = partition::insert(conn, row, &issued)?;
    catalog::record(&tx, &slot, row, &stored)?;
    tx.commit()?;
    Ok(Ref::stored(stored))
}

/// The first candidate of `id` no issued ref in `project` equals.
fn issue(conn: &Connection, project: &str, id: &str) -> Result<String, Error> {
    for candidate in reference::candidates(id) {
        if !catalog::taken(conn, project, candidate)? {
            return Ok(candidate.to_owned());
        }
    }
    Err(Error::Sqlite {
        reason: format!("every ref of {id} is issued in {project}"),
    })
}
