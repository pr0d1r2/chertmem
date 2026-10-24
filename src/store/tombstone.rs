//! Drop tombstones: `<path>.dropped` beside a dropped partition under
//! `chunks/<project>/`, 1 JSON object holding its catalog row and every
//! ref it issued, written (temp + rename) before its files go, so
//! `rebuild` keeps each ref dropped across a lost catalog and never
//! issues it again (src/store V12, R5).

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use rusqlite::types::Value;
use rusqlite::{Connection, params_from_iter};
use serde_json::{Value as Json, json};

use super::{Error, catalog};

const EXT: &str = "dropped";

const ROW: &str = "SELECT path, dims, project, ide, t0, t1, shard, rows,
  bytes, state, checksum, spec FROM partitions WHERE path = ?1";
const REFS: &str = "SELECT r.project, r.ref, r.id, r.identity FROM refs r
  JOIN partitions p ON p.id = r.partition WHERE p.path = ?1 ORDER BY r.id";
const PUT: &str = "INSERT INTO partitions (path, dims, project, ide, t0,
  t1, shard, rows, bytes, state, checksum, spec, dropped)
  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1)
  ON CONFLICT (path) DO UPDATE SET dropped = 1";
const REF: &str = "INSERT OR IGNORE INTO refs
  (project, ref, id, identity, partition) VALUES (?1, ?2, ?3, ?4, ?5)";

/// The tombstone of the partition at `path`.
fn file(root: &Path, path: &str) -> PathBuf {
    root.join(format!("{path}.{EXT}"))
}

/// Whether `file` is a tombstone.
pub(super) fn is(file: &Path) -> bool {
    file.extension().is_some_and(|x| x == EXT)
}

/// Every row `sql` selects for `path`, each a JSON array of its columns.
fn rows(conn: &Connection, sql: &str, path: &str) -> Result<Vec<Json>, Error> {
    let mut stmt = conn.prepare(sql)?;
    let n = stmt.column_count();
    let found = stmt.query_map([path], |r| {
        let cols = (0..n).map(|i| r.get::<_, Value>(i).map(to_json));
        Ok(Json::Array(cols.collect::<Result<_, _>>()?))
    })?;
    Ok(found.collect::<Result<_, _>>()?)
}

fn to_json(v: Value) -> Json {
    match v {
        Value::Integer(i) => Json::from(i),
        Value::Text(s) => Json::from(s),
        _ => Json::Null,
    }
}

fn to_sql(v: &Json) -> Value {
    match v {
        Json::Number(n) => n.as_i64().map_or(Value::Null, Value::Integer),
        Json::String(s) => Value::Text(s.clone()),
        _ => Value::Null,
    }
}

/// Write the tombstone of the partition at `path` from the catalog.
pub(super) fn write(
    conn: &Connection,
    root: &Path,
    path: &str,
) -> Result<(), Error> {
    let partition = rows(conn, ROW, path)?.into_iter().next();
    let stone =
        json!({ "partition": partition, "refs": rows(conn, REFS, path)? });
    let done = file(root, path);
    let tmp = root.join(format!("{path}.{}.tmp", ulid::Ulid::generate()));
    std::fs::write(&tmp, format!("{stone}\n"))
        .map_err(|e| Error::io(&tmp, &e))?;
    std::fs::rename(&tmp, &done).map_err(|e| Error::io(&done, &e))
}

/// A tombstone as read back: the partition's row and its refs.
struct Stone {
    row: Vec<Json>,
    refs: Vec<Json>,
}

/// The tombstone at `file`; one that does not parse is an error, so a
/// rebuild never quietly forgets a drop.
fn read(file: &Path) -> Result<Stone, Error> {
    let bad = || Error::Io {
        path: file.to_path_buf(),
        reason: "unreadable tombstone".to_owned(),
    };
    let text =
        std::fs::read_to_string(file).map_err(|e| Error::io(file, &e))?;
    let mut stone: Json = serde_json::from_str(&text).map_err(|_| bad())?;
    let mut take = |key: &str| match stone.get_mut(key).map(Json::take) {
        Some(Json::Array(items)) => Ok(items),
        _ => Err(bad()),
    };
    Ok(Stone {
        row: take("partition")?,
        refs: take("refs")?,
    })
}

/// Put the tombstone at `file` back in the catalog: its row marked
/// dropped, made if the catalog lost it, and every ref it issued.
pub(super) fn restore(tx: &Connection, file: &Path) -> Result<(), Error> {
    let stone = read(file)?;
    tx.execute(PUT, params_from_iter(stone.row.iter().map(to_sql)))?;
    let path = stone.row.first().and_then(Json::as_str).unwrap_or_default();
    let (id, _) = catalog::state(tx, path)?;
    for r in stone.refs.iter().filter_map(Json::as_array) {
        let args = r.iter().map(to_sql).chain([Value::Integer(id)]);
        tx.execute(REF, params_from_iter(args))?;
    }
    Ok(())
}
