//! Summaries: 1 table per partition, scope turn or session, keyed by
//! session, bodies compressed by the caller like observations' (src/store
//! C, T9); injection reads them (`src/hook`).

#[cfg(test)]
mod tests;

use rusqlite::{Connection, params};

use super::write::Note;
use super::{Error, Filter, Listed, Scope, Store, Summary};

const INSERT: &str = "INSERT INTO summaries
  (id, project, session, scope, ts, ide, body)
  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
  ON CONFLICT (id) DO NOTHING";

/// Insert `note`; an id stored before stores nothing.
pub(super) fn insert(conn: &Connection, note: &Note) -> Result<(), Error> {
    let s = &note.summary;
    let (id, p, scope) = (&note.id, &note.project, s.scope.as_str());
    let args = params![id, p, s.session, scope, s.ts, s.ide, s.body];
    conn.execute(INSERT, args)?;
    Ok(())
}

const ALL: &str = "SELECT ts, session, scope, ide, body, id, project
  FROM summaries ORDER BY seq";

/// Every summary of 1 partition, in insert order, as stored.
pub(super) fn all(conn: &Connection) -> Result<Vec<Note>, Error> {
    let mut stmt = conn.prepare(ALL)?;
    let rows = stmt.query_map([], |r| {
        let scope: String = r.get(2)?;
        // The table's CHECK admits only the 2 names.
        let scope = Scope::parse(&scope).unwrap_or(Scope::Turn);
        Ok(Note {
            id: r.get(5)?,
            project: r.get(6)?,
            summary: summary(r, scope)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Delete the summary `id`, which a repartition moved elsewhere.
pub(super) fn remove(conn: &Connection, id: &str) -> Result<(), Error> {
    conn.execute("DELETE FROM summaries WHERE id = ?1", [id])?;
    Ok(())
}

/// Every session's summaries of 1 scope, through `summaries_scope`; `?2`
/// is bound but unused, so both queries take the same 3 values.
const LIST_ALL: &str = "SELECT ts, session, scope, ide, body, id
  FROM summaries WHERE project = ?1 AND scope = ?3 ORDER BY ts, id";

/// 1 session's, through `summaries_session`.
const LIST_ONE: &str = "SELECT ts, session, scope, ide, body, id
  FROM summaries WHERE project = ?1 AND session = ?2 AND scope = ?3
  ORDER BY ts, id";

/// The query [`list`] runs, of 1 session or of every session: 2, so
/// each searches an index (src/store T19).
fn list_sql(session: Option<&str>) -> &'static str {
    if session.is_some() {
        LIST_ONE
    } else {
        LIST_ALL
    }
}

/// The summaries of 1 partition, with their ids for a stable order.
fn list(
    conn: &Connection,
    project: &str,
    session: Option<&str>,
    scope: Scope,
) -> Result<Vec<Listed>, Error> {
    let mut stmt = conn.prepare(list_sql(session))?;
    let args = params![project, session, scope.as_str()];
    let rows = stmt.query_map(args, |r| {
        Ok(Listed {
            id: r.get(5)?,
            summary: summary(r, scope)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn summary(r: &rusqlite::Row<'_>, scope: Scope) -> rusqlite::Result<Summary> {
    Ok(Summary {
        ts: r.get(0)?,
        session: r.get(1)?,
        scope,
        ide: r.get(3)?,
        body: r.get(4)?,
    })
}

impl Store {
    /// This project's `scope` summaries in the 1 partition at `path`, by
    /// time then id, opening only that file; none when its file is gone
    /// (src/store T19).
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; an unreadable file.
    pub fn summaries_in(
        &mut self,
        path: &str,
        scope: Scope,
    ) -> Result<Vec<Listed>, Error> {
        self.reading()?;
        match self.chunks.existing(path)? {
            Some(conn) => list(conn, &self.project, None, scope),
            None => Ok(Vec::new()),
        }
    }

    /// This project's `scope` summaries, of `session` or of every
    /// session, by time across every partition the catalog names.
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; an unreadable file.
    pub fn summaries(
        &mut self,
        session: Option<&str>,
        scope: Scope,
    ) -> Result<Vec<Summary>, Error> {
        let mine = Filter {
            project: Some(self.project.clone()),
            ..Filter::default()
        };
        let mut found = Vec::new();
        for p in self.partitions(&mine)? {
            if let Some(conn) = self.chunks.existing(&p.path)? {
                found.extend(list(conn, &self.project, session, scope)?);
            }
        }
        found.sort_by(|a, b| (a.summary.ts, &a.id).cmp(&(b.summary.ts, &b.id)));
        Ok(found.into_iter().map(|l| l.summary).collect())
    }
}
