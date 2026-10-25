//! Global term statistics, so BM25 IDF compares across partitions
//! (src/search C, V2): `<data_dir>/search/terms.db`, kept from the
//! store's public reads (src/search R1). Per partition it remembers the
//! last seq counted and tails past it, and the catalog checksum it last
//! saw, so a partition whose checksum stands is not opened at all (T11);
//! a new `generation`, or a counted partition gone from the catalog,
//! recounts from scratch.

#[cfg(test)]
mod tests;

use std::path::Path;
use std::time::Duration;

use rusqlite::{
    Connection, OptionalExtension, Transaction, TransactionBehavior, params,
};

use super::{Error, token};
use crate::store::{Filter, Partition, Store};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS parts (path TEXT PRIMARY KEY, seq INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS terms (term TEXT PRIMARY KEY, df INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS seen (path TEXT PRIMARY KEY, sum TEXT NOT NULL);
";

/// Recounts when the store moves during a refresh, before giving up.
const TRIES: u32 = 3;

/// What BM25 needs of the whole store for 1 query's terms.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Stats {
    /// Rows counted.
    pub(super) docs: u64,
    /// Tokens of every row counted.
    pub(super) tokens: u64,
    /// Rows holding each query term, in the query's term order.
    pub(super) df: Vec<u64>,
}

/// The term file of 1 data dir.
#[derive(Debug)]
pub(super) struct Terms {
    conn: Connection,
}

impl Terms {
    /// Open (or lay) `<data_dir>/search/terms.db`, WAL, so processes
    /// share it.
    pub(super) fn open(data_dir: &Path) -> Result<Self, Error> {
        let dir = data_dir.join("search");
        std::fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, &e))?;
        let conn = Connection::open(dir.join("terms.db"))?;
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    /// Count every row the store holds and this file does not yet.
    pub(super) fn refresh(&mut self, store: &mut Store) -> Result<(), Error> {
        for _ in 0..TRIES {
            if self.sync(store)? {
                return Ok(());
            }
        }
        Err(Error::Terms {
            reason: "the store kept changing during a recount".into(),
        })
    }

    /// 1 pass under the write lock; `false` (rolled back) when the
    /// store's generation moved meanwhile.
    fn sync(&mut self, store: &mut Store) -> Result<bool, Error> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let generation = store.generation()?;
        count_all(&tx, store, generation)?;
        if store.generation()? != generation {
            return Ok(false);
        }
        tx.commit()?;
        Ok(true)
    }

    /// The stats of `terms` as last refreshed.
    pub(super) fn stats(&self, terms: &[String]) -> Result<Stats, Error> {
        let mut stmt =
            self.conn.prepare("SELECT df FROM terms WHERE term = ?1")?;
        let mut df = Vec::with_capacity(terms.len());
        for t in terms {
            df.push(stmt.query_row([t], |r| r.get(0)).optional()?.unwrap_or(0));
        }
        Ok(Stats {
            docs: meta(&self.conn, "docs")?.unwrap_or(0),
            tokens: meta(&self.conn, "tokens")?.unwrap_or(0),
            df,
        })
    }
}

/// Count every partition the catalog lists, from scratch when stale.
fn count_all(
    tx: &Transaction<'_>,
    store: &mut Store,
    generation: u64,
) -> Result<(), Error> {
    let parts = store.partitions(&Filter::default())?;
    if stale(tx, generation, &parts)? {
        clear(tx, generation)?;
    }
    for p in &parts {
        if !seen(tx, p)? {
            absorb(tx, store, &p.path)?;
            see(tx, p)?;
        }
    }
    Ok(())
}

/// Whether `p`'s catalog checksum is the 1 last counted: no row came
/// since (src/store V6: the file is written first), so its file need
/// not be opened (T11, V6).
fn seen(tx: &Transaction<'_>, p: &Partition) -> Result<bool, Error> {
    let sql = "SELECT sum FROM seen WHERE path = ?1";
    let sum: Option<String> =
        tx.query_row(sql, [&p.path], |r| r.get(0)).optional()?;
    Ok(sum.as_ref() == Some(&p.stats.checksum))
}

/// Remember `p`'s catalog checksum as counted.
fn see(tx: &Transaction<'_>, p: &Partition) -> Result<(), Error> {
    let sql = "INSERT INTO seen (path, sum) VALUES (?1, ?2)
      ON CONFLICT (path) DO UPDATE SET sum = excluded.sum";
    tx.execute(sql, [&p.path, &p.stats.checksum])?;
    Ok(())
}

/// Whether the counts no longer match the store: another generation,
/// or a counted partition the catalog no longer lists.
fn stale(
    tx: &Transaction<'_>,
    generation: u64,
    parts: &[Partition],
) -> Result<bool, Error> {
    if meta(tx, "generation")? != Some(generation) {
        return Ok(true);
    }
    let mut stmt = tx.prepare("SELECT path FROM parts")?;
    let counted = stmt.query_map([], |r| r.get::<_, String>(0))?;
    for path in counted {
        let path = path?;
        if !parts.iter().any(|p| p.path == path) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Forget every count; the next passes recount from seq 0.
fn clear(tx: &Transaction<'_>, generation: u64) -> Result<(), Error> {
    tx.execute_batch(
        "DELETE FROM parts; DELETE FROM terms; DELETE FROM meta;
        DELETE FROM seen",
    )?;
    let sql = "INSERT INTO meta (key, value) VALUES ('generation', ?1),
      ('docs', 0), ('tokens', 0)";
    tx.execute(sql, [generation])?;
    Ok(())
}

/// Count the rows of `path` past its last counted seq.
fn absorb(
    tx: &Transaction<'_>,
    store: &mut Store,
    path: &str,
) -> Result<(), Error> {
    let rows = store.tail(path, counted(tx, path)?, &Filter::default())?;
    let Some(last) = rows.last().map(|t| t.seq) else {
        return Ok(());
    };
    for row in &rows {
        count(tx, &row.observation.body)?;
    }
    let sql = "INSERT INTO parts (path, seq) VALUES (?1, ?2)
      ON CONFLICT (path) DO UPDATE SET seq = excluded.seq";
    tx.execute(sql, params![path, last])?;
    Ok(())
}

/// The last seq of `path` counted; 0 before any.
fn counted(tx: &Transaction<'_>, path: &str) -> Result<i64, Error> {
    let sql = "SELECT seq FROM parts WHERE path = ?1";
    let seq = tx.query_row(sql, [path], |r| r.get(0)).optional()?;
    Ok(seq.unwrap_or(0))
}

/// Count 1 row: 1 doc, its tokens, 1 toward each distinct term's df.
fn count(tx: &Transaction<'_>, body: &str) -> Result<(), Error> {
    let tokens = i64::try_from(token::tokens(body).len()).unwrap_or(i64::MAX);
    let sql = "INSERT INTO terms (term, df) VALUES (?1, 1)
      ON CONFLICT (term) DO UPDATE SET df = df + 1";
    let mut stmt = tx.prepare(sql)?;
    for term in token::terms(body) {
        stmt.execute([term])?;
    }
    let sql = "UPDATE meta SET value = value + CASE key
      WHEN 'docs' THEN 1 ELSE ?1 END WHERE key IN ('docs', 'tokens')";
    tx.execute(sql, [tokens])?;
    Ok(())
}

fn meta(conn: &Connection, key: &str) -> Result<Option<u64>, Error> {
    let sql = "SELECT value FROM meta WHERE key = ?1";
    Ok(conn.query_row(sql, [key], |r| r.get(0)).optional()?)
}
