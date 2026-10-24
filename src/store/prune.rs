//! Partition pruning: a read picks partitions from the catalog's dims
//! and `[t0, t1)` bounds only, never from the current config, so a
//! config change never misroutes a read (src/store V4, root V4).

#[cfg(test)]
mod tests;

use rusqlite::{Connection, params};

use super::{Error, Filter, Partition, State, Stats};

const PARTITIONS: &str = "SELECT path, dims, project, ide, t0, t1, shard,
  rows, bytes, checksum, state, spec FROM partitions
  WHERE (?1 IS NULL OR project = ?1 OR project = '')
    AND (?2 IS NULL OR t1 > ?2)
    AND (?3 IS NULL OR t0 < ?3)
  ORDER BY t0, path";

/// The partitions that may hold a row `filter` wants: same project (or
/// no project dim) and bounds overlapping `[since, until)`.
pub(super) fn partitions(
    conn: &Connection,
    filter: &Filter,
) -> Result<Vec<Partition>, Error> {
    let mut stmt = conn.prepare(PARTITIONS)?;
    let args = params![filter.project, filter.since, filter.until];
    let rows = stmt.query_map(args, partition)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn partition(r: &rusqlite::Row<'_>) -> rusqlite::Result<Partition> {
    let state: String = r.get(10)?;
    Ok(Partition {
        path: r.get(0)?,
        dims: r.get(1)?,
        project: r.get(2)?,
        ide: r.get(3)?,
        t0: r.get(4)?,
        t1: r.get(5)?,
        shard: r.get(6)?,
        stats: stats(r)?,
        // The catalog's CHECK admits only the 3 names.
        state: State::parse(&state).unwrap_or(State::Hot),
        spec: r.get(11)?,
    })
}

fn stats(r: &rusqlite::Row<'_>) -> rusqlite::Result<Stats> {
    Ok(Stats {
        rows: r.get(7)?,
        bytes: r.get(8)?,
        checksum: r.get(9)?,
    })
}
