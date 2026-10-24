//! 1 append: identity check, ref issue, partition insert, then the
//! catalog, all under the catalog's write lock (src/store V6, V9).

#[cfg(test)]
mod tests;

use rusqlite::{Connection, Transaction, TransactionBehavior};

use super::catalog::{self, Slot};
use super::layout::{Key, Plan};
use super::partition::{self, Chunks};
use super::reference;
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
    fn key(&self) -> Key<'_> {
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
    fn key(&self) -> Key<'_> {
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
}

/// Store a replayed item, whichever it is.
pub(super) fn apply(cx: &mut Ctx<'_>, item: &Item) -> Result<(), Error> {
    match item {
        Item::Observation(row) => observation(cx, row).map(drop),
        Item::Summary(note) => summary(cx, note),
    }
}

/// Store `note` in the hot partition of its own timestamp; its id seen
/// before stores nothing (a replayed spool file).
pub(super) fn summary(cx: &mut Ctx<'_>, note: &Note) -> Result<(), Error> {
    let tx = immediate(cx.catalog)?;
    let slot = slot(cx.plan, &tx, note.key())?;
    summary::insert(cx.chunks.get(&slot.path)?, note)?;
    Ok(tx.commit()?)
}

/// What a write reaches.
#[derive(Debug)]
pub(super) struct Ctx<'a> {
    pub catalog: &'a Connection,
    pub chunks: &'a mut Chunks,
    pub plan: &'a Plan,
}

/// A new ULID at `ts`, so the id sorts by the event's own time.
pub(super) fn new_id(ts: u64) -> String {
    ulid::Ulid::from_parts(ts, ulid::Ulid::generate().random()).to_string()
}

/// The catalog's write lock: 1 writer across processes, the busy timeout
/// then [`Error::Busy`].
fn immediate(conn: &Connection) -> Result<Transaction<'_>, Error> {
    let behavior = TransactionBehavior::Immediate;
    Ok(Transaction::new_unchecked(conn, behavior)?)
}

/// The hot partition `key` lands in, in the catalog.
fn slot(plan: &Plan, conn: &Connection, key: Key<'_>) -> Result<Slot, Error> {
    let place = plan.place(key)?;
    catalog::ensure(conn, &place, &plan.dims(), &plan.spec())
}

/// Store `row`; its ref, the first one for a known id or identity.
pub(super) fn observation(cx: &mut Ctx<'_>, row: &Row) -> Result<Ref, Error> {
    let tx = immediate(cx.catalog)?;
    if let Some(found) = catalog::existing(&tx, row)? {
        return Ok(Ref::stored(found));
    }
    let slot = slot(cx.plan, &tx, row.key())?;
    let issued = issue(&tx, &row.project, &row.id)?;
    let stored = partition::insert(cx.chunks.get(&slot.path)?, row, &issued)?;
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
