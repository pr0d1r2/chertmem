//! `catalog.db`: 1 row per partition and every issued ref; every write
//! here follows its partition write (src/store C, V6).

#[cfg(test)]
mod tests;

use rusqlite::{Connection, OptionalExtension, params};

use super::layout::{self, FNV_OFFSET, Place};
use super::write::Row;
use super::{Error, State};

/// A partition's catalog row, as a write needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Slot {
    pub id: i64,
    pub path: String,
}

const ENSURE: &str = "INSERT INTO partitions
  (path, dims, project, ide, t0, t1, shard, checksum, spec)
  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
  ON CONFLICT (path) DO NOTHING";

/// The catalog row of `place`, made on first use; a warm or cold
/// partition refuses the write (src/store V7).
pub(super) fn ensure(
    conn: &Connection,
    place: &Place,
    dims: &str,
    spec: &str,
) -> Result<Slot, Error> {
    let p = place;
    let sum = format!("{FNV_OFFSET:016x}");
    let (t0, t1) = (p.t0, p.t1);
    let row =
        params![p.path, dims, p.project, p.ide, t0, t1, p.shard, sum, spec];
    conn.execute(ENSURE, row)?;
    let sql = "SELECT id, state FROM partitions WHERE path = ?1";
    let (id, state): (i64, String) =
        conn.query_row(sql, [&p.path], |r| Ok((r.get(0)?, r.get(1)?)))?;
    let path = p.path.clone();
    if State::parse(&state) == Some(State::Hot) {
        Ok(Slot { id, path })
    } else {
        Err(Error::ReadOnly { path })
    }
}

/// The ref already issued to this row's id or its identity (root V16).
pub(super) fn existing(
    conn: &Connection,
    row: &Row,
) -> Result<Option<String>, Error> {
    let sql = "SELECT ref FROM refs
      WHERE id = ?1 OR (project = ?2 AND identity = ?3) LIMIT 1";
    let args = params![row.id, row.project, row.event.identity];
    Ok(conn.query_row(sql, args, |r| r.get(0)).optional()?)
}

/// Whether `candidate` is an issued ref in `project` (root V15).
pub(super) fn taken(
    conn: &Connection,
    project: &str,
    candidate: &str,
) -> Result<bool, Error> {
    let sql = "SELECT 1 FROM refs WHERE project = ?1 AND ref = ?2";
    let hit: Option<i64> = conn
        .query_row(sql, [project, candidate], |r| r.get(0))
        .optional()?;
    Ok(hit.is_some())
}

/// Record a stored row: its ref, and the partition's rows, bytes and
/// checksum (src/store R1).
pub(super) fn record(
    conn: &Connection,
    slot: &Slot,
    row: &Row,
    stored: &str,
) -> Result<(), Error> {
    let sql = "INSERT INTO refs (project, ref, id, identity, partition)
      VALUES (?1, ?2, ?3, ?4, ?5)";
    let identity = &row.event.identity;
    conn.execute(sql, params![row.project, stored, row.id, identity, slot.id])?;
    let sql = "SELECT checksum FROM partitions WHERE id = ?1";
    let sum: String = conn.query_row(sql, [slot.id], |r| r.get(0))?;
    let next = checksum(&sum, &row.id, &row.event.body);
    let bytes = u64::try_from(row.event.body.len()).unwrap_or(u64::MAX);
    let sql = "UPDATE partitions SET rows = rows + 1, bytes = bytes + ?2,
      checksum = ?3 WHERE id = ?1";
    conn.execute(sql, params![slot.id, bytes, next])?;
    Ok(())
}

/// FNV-1a 64 continued from `sum` over the row's id and body, so the
/// checksum folds every row in insert order.
fn checksum(sum: &str, id: &str, body: &str) -> String {
    let state = u64::from_str_radix(sum, 16).unwrap_or(FNV_OFFSET);
    let state = layout::fnv1a(state, id.as_bytes());
    let state = layout::fnv1a(layout::fnv1a(state, &[0x1f]), body.as_bytes());
    format!("{:016x}", layout::fnv1a(state, &[0x1e]))
}
