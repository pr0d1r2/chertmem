//! FTS5 candidates for `src/search` (src/store T17, R8): the rows of 1
//! partition whose index matches any term, and the index's counts for
//! IDF, so a query never scans every row.

#[cfg(test)]
mod tests;

use rusqlite::{Connection, OptionalExtension, params};

use super::partition::observation;
use super::{Error, Filter, Frequencies, Store, Tailed};

/// `terms` as 1 FTS5 OR query: each a string in double quotes, any `"`
/// in it doubled, so its text is only ever a phrase and never FTS5
/// syntax; `None` when every term is blank.
fn query(terms: &[String]) -> Option<String> {
    let quoted: Vec<String> = (terms.iter())
        .filter(|t| !t.trim().is_empty())
        .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
        .collect();
    (!quoted.is_empty()).then(|| quoted.join(" OR "))
}

const MATCH: &str = "SELECT ref, id, project, ts, session, ide, kind,
  identity, body, seq FROM observations
  WHERE seq IN (SELECT rowid FROM observations_fts
                WHERE observations_fts MATCH ?1)
    AND (?2 IS NULL OR project = ?2)
    AND (?3 IS NULL OR ts >= ?3)
    AND (?4 IS NULL OR ts < ?4)
  ORDER BY seq";

/// The rows of 1 partition matching `query` that `filter` wants.
fn rows(
    conn: &Connection,
    query: &str,
    filter: &Filter,
) -> Result<Vec<Tailed>, Error> {
    let mut stmt = conn.prepare(MATCH)?;
    let f = filter;
    let args = params![query, f.project, f.since, f.until];
    let found = stmt.query_map(args, |r| {
        Ok(Tailed {
            seq: r.get(9)?,
            observation: observation(r)?,
        })
    })?;
    Ok(found.collect::<Result<_, _>>()?)
}

/// A per-connection view of the index's terms; `temp`, so the file
/// itself, warm or cold, is never written.
const VOCAB: &str = "CREATE VIRTUAL TABLE IF NOT EXISTS temp.chert_vocab
  USING fts5vocab(main, observations_fts, row)";

const DOCS: &str = "SELECT doc FROM temp.chert_vocab WHERE term = ?1";

/// The row count and the rows per term of 1 partition.
fn count(conn: &Connection, terms: &[String]) -> Result<Frequencies, Error> {
    conn.execute_batch(VOCAB)?;
    let sql = "SELECT count(*) FROM observations";
    let rows = conn.query_row(sql, [], |r| r.get(0))?;
    let mut stmt = conn.prepare(DOCS)?;
    let mut docs = Vec::with_capacity(terms.len());
    for term in terms {
        let n = stmt.query_row([term.to_lowercase()], |r| r.get(0));
        docs.push(n.optional()?.unwrap_or(0));
    }
    Ok(Frequencies { rows, docs })
}

impl Store {
    /// The rows of the partition at `path` (as in
    /// [`super::Partition::path`]) whose FTS5 index matches any of
    /// `terms` and that `filter` wants, by seq (src/store T17). Each
    /// term is a quoted phrase, never FTS5 syntax: `a*` is the token
    /// `a`, `col:x` the phrase `col x`. Blank terms are skipped; none
    /// left, or a gone file, match nothing.
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; an unreadable file.
    pub fn matches(
        &mut self,
        path: &str,
        terms: &[String],
        filter: &Filter,
    ) -> Result<Vec<Tailed>, Error> {
        self.reading()?;
        let Some(query) = query(terms) else {
            return Ok(Vec::new());
        };
        match self.chunks.existing(path)? {
            Some(conn) => rows(conn, &query, filter),
            None => Ok(Vec::new()),
        }
    }

    /// For IDF: the indexed rows of the partition at `path`, of every
    /// project, and per term the rows holding it, read from `fts5vocab`
    /// (src/store T17). A term counts as 1 token of the index's
    /// `unicode61` tokenizer, matched lowercased; anything else, such as
    /// 2 words, counts 0. A gone file counts 0 everywhere.
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; an unreadable file.
    pub fn frequencies(
        &mut self,
        path: &str,
        terms: &[String],
    ) -> Result<Frequencies, Error> {
        self.reading()?;
        match self.chunks.existing(path)? {
            Some(conn) => count(conn, terms),
            None => Ok(Frequencies {
                rows: 0,
                docs: vec![0; terms.len()],
            }),
        }
    }
}
