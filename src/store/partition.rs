//! Partition files: 1 connection each, opened on first use and kept,
//! at most [`Store::KEEP_OPEN`] a store with the least recently used
//! closed first (src/store C, T20, root R5), and the rows they hold.

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, params};

use super::layout::{FNV_OFFSET, Place, Plan};
use super::write::Row;
use super::{
    Error, Filter, Observation, Ref, State, Stats, Store, catalog, cold, schema,
};

impl Store {
    /// The most partition connections a store keeps open unless told
    /// otherwise; each holds up to 3 files (the file, `-wal`, `-shm`),
    /// so 1 store stays under a 256 file limit (src/store T20).
    pub const KEEP_OPEN: usize = 64;

    /// Keep at most `n` partition connections open from now on, the
    /// least recently used closed first; the 1 in use stays open even
    /// when `n` is 0 (src/store T20, src/search V6).
    pub fn keep_open(&mut self, n: usize) {
        self.chunks.keep(n);
    }
}

/// 1 open partition file and when it was last used.
#[derive(Debug)]
struct Kept {
    conn: Connection,
    used: u64,
}

/// The open partition files of 1 store.
#[derive(Debug)]
pub(super) struct Chunks {
    root: PathBuf,
    busy: Duration,
    open: BTreeMap<String, Kept>,
    keep: usize,
    clock: u64,
}

impl Chunks {
    pub(super) fn new(root: PathBuf, busy: Duration) -> Self {
        Self {
            root,
            busy,
            open: BTreeMap::new(),
            keep: Store::KEEP_OPEN,
            clock: 0,
        }
    }

    /// Keep at most `n` connections, the least recent closed now.
    pub(super) fn keep(&mut self, n: usize) {
        self.keep = n;
        self.trim(n);
    }

    /// Close the least recently used connections until `n` are left.
    fn trim(&mut self, n: usize) {
        while let Some(path) = self.oldest().filter(|_| self.open.len() > n) {
            self.open.remove(&path);
        }
    }

    /// The path of the least recently used connection.
    fn oldest(&self) -> Option<String> {
        let oldest = self.open.iter().min_by_key(|(_, k)| k.used);
        oldest.map(|(p, _)| p.clone())
    }

    /// The partition at `path` (relative to the data dir), made when new.
    pub(super) fn get(&mut self, path: &str) -> Result<&Connection, Error> {
        let file = self.root.join(path);
        self.open_at(path, &file)
    }

    /// The connection kept under `path`, opened on `file` first.
    fn open_at(
        &mut self,
        path: &str,
        file: &Path,
    ) -> Result<&Connection, Error> {
        self.clock = self.clock.wrapping_add(1);
        if let Some(kept) = self.open.get_mut(path) {
            kept.used = self.clock;
        } else {
            cold::dir_of(file)?;
            let conn = schema::open(file, self.busy, schema::PARTITION)?;
            self.trim(self.keep.saturating_sub(1));
            let used = self.clock;
            self.open.insert(path.to_owned(), Kept { conn, used });
        }
        let kept = self.open.get(path).map(|k| &k.conn);
        kept.ok_or(Error::NoDataDir)
    }

    /// The partition at `path` when its file exists, a cold one through
    /// its thawed copy; a read never makes one, so a dropped file reads
    /// as gone.
    pub(super) fn existing(
        &mut self,
        path: &str,
    ) -> Result<Option<&Connection>, Error> {
        if self.open.contains_key(path) || self.root.join(path).is_file() {
            return self.get(path).map(Some);
        }
        if !cold::zst(&self.root, path).is_file() {
            return Ok(None);
        }
        let thawed = cold::thaw(&self.root, path)?;
        self.open_at(path, &thawed).map(Some)
    }

    /// The hot file of `place` for a write, stamped with its catalog row
    /// on first use here; a cold or sealed file refuses even when a
    /// stale catalog calls it hot (src/store V6, V7).
    pub(super) fn hot(
        &mut self,
        place: &Place,
        dims: &str,
        spec: &str,
    ) -> Result<&Connection, Error> {
        let path = &place.path;
        let first = !self.open.contains_key(path);
        if first && cold::zst(&self.root, path).is_file() {
            return Err(Error::ReadOnly { path: path.clone() });
        }
        let conn = self.get(path)?;
        if first {
            stamp(conn, place, dims, spec)?;
        }
        Ok(conn)
    }

    /// Close the connection kept under `path`, before its file goes.
    pub(super) fn close(&mut self, path: &str) {
        self.open.remove(path);
    }
}

const STAMP: &str = "INSERT INTO chunk
  (one, dims, project, ide, t0, t1, shard, spec)
  VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)
  ON CONFLICT (one) DO NOTHING";

/// Stamp a new file with its catalog row; a file stamped sealed refuses.
fn stamp(
    conn: &Connection,
    place: &Place,
    dims: &str,
    spec: &str,
) -> Result<(), Error> {
    let p = place;
    let args = params![dims, p.project, p.ide, p.t0, p.t1, p.shard, spec];
    conn.execute(STAMP, args)?;
    let sql = "SELECT state FROM chunk";
    let state: String = conn.query_row(sql, [], |r| r.get(0))?;
    if State::parse(&state) == Some(State::Hot) {
        Ok(())
    } else {
        Err(Error::ReadOnly {
            path: p.path.clone(),
        })
    }
}

/// Record the tier in the file itself (src/store V6).
pub(super) fn set_state(conn: &Connection, state: State) -> Result<(), Error> {
    conn.execute("UPDATE chunk SET state = ?1", [state.as_str()])?;
    Ok(())
}

/// Unbounded ends read as 0 and `i64::MAX`, so `SQLite` searches
/// `observations_ts` by the range; `(? IS NULL OR ts >= ?)` it cannot
/// (src/store T19).
const SCAN: &str = "SELECT ref, id, project, ts, session, ide, kind,
  identity, body FROM observations
  WHERE (?1 IS NULL OR project = ?1)
    AND ts >= coalesce(?2, 0)
    AND ts < coalesce(?3, 9223372036854775807)
  ORDER BY ts, id";

/// The rows of 1 partition `filter` wants, by time then id.
pub(super) fn scan(
    conn: &Connection,
    filter: &Filter,
) -> Result<Vec<Observation>, Error> {
    let mut stmt = conn.prepare(SCAN)?;
    let args = params![filter.project, filter.since, filter.until];
    let rows = stmt.query_map(args, observation)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

const FETCH: &str = "SELECT ref, id, project, ts, session, ide, kind,
  identity, body FROM observations WHERE project = ?1 AND ref = ?2";

/// The row stored under exactly `reference` in `project`.
pub(super) fn fetch(
    conn: &Connection,
    project: &str,
    reference: &str,
) -> Result<Option<Observation>, Error> {
    let args = [project, reference];
    Ok(conn.query_row(FETCH, args, observation).optional()?)
}

/// An observation from the columns `ref, id, project, ts, session, ide,
/// kind, identity, body`.
pub(super) fn observation(
    r: &rusqlite::Row<'_>,
) -> rusqlite::Result<Observation> {
    Ok(Observation {
        reference: Ref::stored(r.get(0)?),
        id: r.get(1)?,
        project: r.get(2)?,
        ts: r.get(3)?,
        session: r.get(4)?,
        ide: r.get(5)?,
        kind: r.get(6)?,
        identity: r.get(7)?,
        body: r.get(8)?,
    })
}

const INSERT: &str = "INSERT INTO observations
  (id, ref, project, identity, ts, session, ide, kind, body)
  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
  ON CONFLICT DO NOTHING";

const STORED: &str = "SELECT ref FROM observations
  WHERE id = ?1 OR (project = ?2 AND identity = ?3)";

/// Insert `row` under `issued`; the ref the partition holds for it, the
/// earlier one when a crash left the row here but not in the catalog.
pub(super) fn insert(
    conn: &Connection,
    row: &Row,
    issued: &str,
) -> Result<String, Error> {
    let (e, id, p) = (&row.event, &row.id, &row.project);
    let args = params![
        id, issued, p, e.identity, e.ts, e.session, e.ide, e.kind, e.body
    ];
    conn.execute(INSERT, args)?;
    let key = params![id, p, e.identity];
    Ok(conn.query_row(STORED, key, |r| r.get(0))?)
}

/// Delete the row `id`, which a repartition moved elsewhere.
pub(super) fn remove(conn: &Connection, id: &str) -> Result<(), Error> {
    conn.execute("DELETE FROM observations WHERE id = ?1", [id])?;
    Ok(())
}

/// The file now holds rows under `plan`'s dims and spec.
pub(super) fn restamp(conn: &Connection, plan: &Plan) -> Result<(), Error> {
    let sql = "UPDATE chunk SET dims = ?1, spec = ?2";
    conn.execute(sql, [plan.dims(), plan.spec()])?;
    Ok(())
}

/// Rows, bytes and checksum of the file, folded in insert (seq) order
/// as the catalog folds them (src/store R1).
pub(super) fn stats(conn: &Connection) -> Result<Stats, Error> {
    let sql = "SELECT id, body FROM observations ORDER BY seq";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    let empty = Stats {
        rows: 0,
        bytes: 0,
        checksum: format!("{FNV_OFFSET:016x}"),
    };
    rows.into_iter().try_fold(empty, |s, row| {
        let (id, body): (String, String) = row?;
        Ok(fold(&s, &id, &body))
    })
}

/// `s` with 1 more row folded in.
fn fold(s: &Stats, id: &str, body: &str) -> Stats {
    let bytes = u64::try_from(body.len()).unwrap_or(u64::MAX);
    Stats {
        rows: s.rows.saturating_add(1),
        bytes: s.bytes.saturating_add(bytes),
        checksum: catalog::checksum(&s.checksum, id, body),
    }
}
