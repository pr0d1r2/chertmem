//! `doctor`: rebuild `catalog.db` from the partition files, each of
//! which holds its own catalog row, so a deleted or stale catalog comes
//! back equal to the one before the crash (src/store T7, V6). A dropped
//! partition has no file: its tombstone holds its row and refs (V12),
//! so a stale or deleted catalog gets them back, and a row whose file
//! is gone is marked dropped.

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

use super::partition::{self, Chunks};
use super::write::immediate;
use super::{Error, State, Stats, Store, catalog, generation, tombstone};

/// A file's own catalog row, its `chunk` table, under its path.
struct Meta {
    path: String,
    dims: String,
    project: String,
    ide: String,
    t0: u64,
    t1: u64,
    shard: Option<u32>,
    spec: String,
    state: String,
}

const META: &str = "SELECT dims, project, ide, t0, t1, shard, spec, state
  FROM chunk";

fn meta(path: &str, r: &rusqlite::Row<'_>) -> rusqlite::Result<Meta> {
    Ok(Meta {
        path: path.to_owned(),
        dims: r.get(0)?,
        project: r.get(1)?,
        ide: r.get(2)?,
        t0: r.get(3)?,
        t1: r.get(4)?,
        shard: r.get(5)?,
        spec: r.get(6)?,
        state: r.get(7)?,
    })
}

/// Every file under `chunks/`, sorted.
fn files(root: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut found = Vec::new();
    let mut dirs = vec![root.join("chunks")];
    while let Some(dir) = dirs.pop() {
        let entries =
            std::fs::read_dir(&dir).map_err(|e| Error::io(&dir, &e))?;
        for path in entries.filter_map(Result::ok).map(|e| e.path()) {
            if path.is_dir() {
                dirs.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Every partition file by its catalog path, `true` when only its
/// `.zst` is there.
fn walk(root: &Path, files: &[PathBuf]) -> BTreeMap<String, bool> {
    let mut found = BTreeMap::new();
    for (name, cold) in files.iter().filter_map(|f| logical(root, f)) {
        let hot = found.get(&name).is_some_and(|c: &bool| !c);
        found.insert(name, cold && !hot);
    }
    found
}

/// The catalog path of a `.db` or `.db.zst` file, and whether it is the
/// `.zst`; `None` for anything else (WAL files, dictionaries, temps).
fn logical(root: &Path, file: &Path) -> Option<(String, bool)> {
    let rel = file.strip_prefix(root).ok()?.to_str()?;
    let (base, cold) = match rel.strip_suffix(".zst") {
        Some(base) => (base, true),
        None => (rel, false),
    };
    let db = Path::new(base).extension().is_some_and(|x| x == "db");
    db.then(|| (base.to_owned(), cold))
}

const LIVE: &str = "SELECT path FROM partitions WHERE dropped = 0";
const UNREF: &str = "DELETE FROM refs
  WHERE partition = (SELECT id FROM partitions WHERE path = ?1)";

/// Clear every live row a file will restore; mark the rest dropped.
fn settle(
    tx: &Connection,
    files: &BTreeMap<String, bool>,
) -> Result<(), Error> {
    let mut stmt = tx.prepare(LIVE)?;
    let live = stmt.query_map([], |r| r.get::<_, String>(0))?;
    for path in live.collect::<Result<Vec<_>, _>>()? {
        if files.contains_key(&path) {
            tx.execute(UNREF, [&path])?;
            catalog::forget(tx, &path)?;
        } else {
            catalog::mark_dropped(tx, &path)?;
        }
    }
    Ok(())
}

/// The catalog row and refs of the file at `path`; a dropped row, a
/// gone file or one without its own row restores nothing.
fn restore(
    tx: &Connection,
    chunks: &mut Chunks,
    path: &str,
    cold: bool,
) -> Result<(), Error> {
    let tombstone = catalog::find(tx, path)?.is_some();
    let Some(conn) = chunks.existing(path)?.filter(|_| !tombstone) else {
        return Ok(());
    };
    let found = conn.query_row(META, [], |r| meta(path, r)).optional()?;
    let Some(mut m) = found else { return Ok(()) };
    if cold {
        State::Cold.as_str().clone_into(&mut m.state);
    }
    let id = insert(tx, &m, &partition::stats(conn)?)?;
    refs(tx, conn, id)
}

const INSERT: &str = "INSERT INTO partitions (path, dims, project, ide,
  t0, t1, shard, rows, bytes, state, checksum, spec)
  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)";

fn insert(tx: &Connection, m: &Meta, s: &Stats) -> Result<i64, Error> {
    let args = params![
        m.path, m.dims, m.project, m.ide, m.t0, m.t1, m.shard, s.rows, s.bytes,
        m.state, s.checksum, m.spec
    ];
    tx.execute(INSERT, args)?;
    Ok(tx.last_insert_rowid())
}

const REFS: &str = "SELECT project, ref, id, identity FROM observations
  ORDER BY seq";
const REF: &str = "INSERT OR IGNORE INTO refs
  (project, ref, id, identity, partition) VALUES (?1, ?2, ?3, ?4, ?5)";

/// Every row's ref, issued again exactly as the file holds it.
fn refs(tx: &Connection, conn: &Connection, id: i64) -> Result<(), Error> {
    type Issued = (String, String, String, Option<String>);
    let mut stmt = conn.prepare(REFS)?;
    let rows = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
    for row in rows {
        let (project, reference, row_id, identity): Issued = row?;
        tx.execute(REF, params![project, reference, row_id, identity, id])?;
    }
    Ok(())
}

impl Store {
    /// Rebuild the catalog from the files under `chunks/`, as `chert
    /// doctor` does after a crash: every file's own row, its stats and
    /// refs, in 1 transaction; then bump `generation` (D15).
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; an unreadable file.
    pub fn rebuild(&mut self) -> Result<(), Error> {
        self.reading()?;
        let catalog = self.catalog.as_ref().ok_or(Error::WriteOnly)?;
        let tx = immediate(catalog)?;
        let all = files(&self.root)?;
        let files = walk(&self.root, &all);
        settle(&tx, &files)?;
        for stone in all.iter().filter(|f| tombstone::is(f)) {
            tombstone::restore(&tx, stone)?;
        }
        for (path, cold) in &files {
            restore(&tx, &mut self.chunks, path, *cold)?;
        }
        generation::bump(&tx, &self.root)?;
        Ok(tx.commit()?)
    }
}
