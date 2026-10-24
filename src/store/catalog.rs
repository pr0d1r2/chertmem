//! `catalog.db`: 1 row per partition and every issued ref; every write
//! here follows its partition write (src/store C, V6).

#[cfg(test)]
mod tests;

use rusqlite::{Connection, OptionalExtension, params};

use super::layout::{self, FNV_OFFSET, Place, Plan};
use super::write::Row;
use super::{Error, State, Stats};

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
    let (id, state) = state(conn, &p.path)?;
    let path = p.path.clone();
    if state == Some(State::Hot) {
        Ok(Slot { id, path })
    } else {
        Err(Error::ReadOnly { path })
    }
}

const STATE: &str = "SELECT id, state, dropped FROM partitions
  WHERE path = ?1";

/// The id and tier of the partition at `path`, the tier `None` once it
/// is dropped; no such row is an error naming it.
pub(super) fn state(
    conn: &Connection,
    path: &str,
) -> Result<(i64, Option<State>), Error> {
    find(conn, path)?.ok_or_else(|| Error::Sqlite {
        reason: format!("no partition {path}"),
    })
}

/// As [`state`], `None` when the catalog has no such row.
pub(super) fn find(
    conn: &Connection,
    path: &str,
) -> Result<Option<(i64, Option<State>)>, Error> {
    let row = |r: &rusqlite::Row<'_>| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get(2)?))
    };
    let found = conn.query_row(STATE, [path], row).optional()?;
    Ok(found.map(|(id, state, dropped)| {
        (id, if dropped { None } else { State::parse(&state) })
    }))
}

/// Move the partition at `path` to `state` (src/store T6).
pub(super) fn set_state(
    conn: &Connection,
    path: &str,
    state: State,
) -> Result<(), Error> {
    let sql = "UPDATE partitions SET state = ?2 WHERE path = ?1";
    conn.execute(sql, [path, state.as_str()])?;
    Ok(())
}

/// Mark the partition at `path` dropped: its row and refs stay, so no
/// ref is issued again and each answers dropped (root V5).
pub(super) fn mark_dropped(conn: &Connection, path: &str) -> Result<(), Error> {
    let sql = "UPDATE partitions SET dropped = 1 WHERE path = ?1";
    conn.execute(sql, [path])?;
    Ok(())
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

/// Where a ref was issued: its project and its partition's file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Hit {
    pub project: String,
    pub path: String,
    /// The partition was dropped: the ref answers dropped (root V5).
    pub dropped: bool,
}

const LOCATE: &str = "SELECT r.project, p.path, p.dropped FROM refs r
  JOIN partitions p ON p.id = r.partition
  WHERE r.ref = ?1 AND (?2 IS NULL OR r.project = ?2)
  ORDER BY r.project";

/// Every partition holding the ref `reference` exactly, in `project` or
/// in every project (root V15); never a prefix match.
pub(super) fn locate(
    conn: &Connection,
    project: Option<&str>,
    reference: &str,
) -> Result<Vec<Hit>, Error> {
    let mut stmt = conn.prepare(LOCATE)?;
    let hits = stmt.query_map(params![reference, project], |r| {
        Ok(Hit {
            project: r.get(0)?,
            path: r.get(1)?,
            dropped: r.get(2)?,
        })
    })?;
    Ok(hits.collect::<Result<_, _>>()?)
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
    grow(conn, slot, row)
}

/// Point the ref of a row a repartition moved at its new partition,
/// and fold the row in there (src/store V5).
pub(super) fn moved(
    conn: &Connection,
    slot: &Slot,
    row: &Row,
) -> Result<(), Error> {
    let sql = "UPDATE refs SET partition = ?2 WHERE id = ?1";
    conn.execute(sql, params![row.id, slot.id])?;
    grow(conn, slot, row)
}

/// Fold `row` into its partition's rows, bytes and checksum.
fn grow(conn: &Connection, slot: &Slot, row: &Row) -> Result<(), Error> {
    let sql = "SELECT checksum FROM partitions WHERE id = ?1";
    let sum: String = conn.query_row(sql, [slot.id], |r| r.get(0))?;
    let next = checksum(&sum, &row.id, &row.event.body);
    let bytes = u64::try_from(row.event.body.len()).unwrap_or(u64::MAX);
    let sql = "UPDATE partitions SET rows = rows + 1, bytes = bytes + ?2,
      checksum = ?3 WHERE id = ?1";
    conn.execute(sql, params![slot.id, bytes, next])?;
    Ok(())
}

const STALE: &str = "SELECT path, state FROM partitions
  WHERE dropped = 0 AND spec <> ?1 ORDER BY t0, path";

/// Every live partition written under a spec other than `spec`, by
/// `t0` then path, with its tier.
pub(super) fn stale(
    conn: &Connection,
    spec: &str,
) -> Result<Vec<(String, State)>, Error> {
    let mut stmt = conn.prepare(STALE)?;
    let rows = stmt.query_map([spec], |r| {
        let state: String = r.get(1)?;
        Ok((r.get(0)?, State::parse(&state).unwrap_or(State::Hot)))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The partition at `path` now holds `stats`, under `plan`'s spec.
pub(super) fn restate(
    conn: &Connection,
    path: &str,
    plan: &Plan,
    stats: &Stats,
) -> Result<(), Error> {
    let sql = "UPDATE partitions SET dims = ?2, spec = ?3, rows = ?4,
      bytes = ?5, checksum = ?6 WHERE path = ?1";
    let (dims, spec, s) = (plan.dims(), plan.spec(), stats);
    let args = params![path, dims, spec, s.rows, s.bytes, s.checksum];
    conn.execute(sql, args)?;
    Ok(())
}

const FORGET: &str = "DELETE FROM partitions WHERE path = ?1
  AND NOT EXISTS (SELECT 1 FROM refs r WHERE r.partition = partitions.id)";

/// Forget the partition at `path` once no ref points at it; one whose
/// refs stay (its file was lost) is marked dropped instead (root V5).
pub(super) fn forget(conn: &Connection, path: &str) -> Result<(), Error> {
    if conn.execute(FORGET, [path])? == 0 {
        mark_dropped(conn, path)?;
    }
    Ok(())
}

/// FNV-1a 64 continued from `sum` over the row's id and body, so the
/// checksum folds every row in insert order.
pub(super) fn checksum(sum: &str, id: &str, body: &str) -> String {
    let state = u64::from_str_radix(sum, 16).unwrap_or(FNV_OFFSET);
    let state = layout::fnv1a(state, id.as_bytes());
    let state = layout::fnv1a(layout::fnv1a(state, &[0x1f]), body.as_bytes());
    format!("{:016x}", layout::fnv1a(state, &[0x1e]))
}
