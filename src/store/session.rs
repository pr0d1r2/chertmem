//! Sessions: 1 catalog row per project and session id, as a session
//! spans buckets; the hook records its start and end, `src/mcp` lists
//! them (src/store T16, R7).

#[cfg(test)]
mod tests;

use rusqlite::{Connection, params};

use super::write::Item;
use super::{Error, Session, SessionStart, Store, Written};

/// A session start with the project it belongs to, as spooled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Begun {
    pub project: String,
    pub start: SessionStart,
}

/// A session end with the project it belongs to, as spooled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Ended {
    pub project: String,
    pub id: String,
    pub ts: u64,
}

/// The earliest start wins, so a resumed session keeps its first.
const START: &str = "INSERT INTO sessions (project, id, ide, cwd, started_at)
  VALUES (?1, ?2, ?3, ?4, ?5)
  ON CONFLICT (project, id) DO UPDATE SET ide = excluded.ide,
    cwd = excluded.cwd, started_at = excluded.started_at
  WHERE started_at IS NULL OR excluded.started_at < started_at";

/// The latest end wins.
const END: &str = "INSERT INTO sessions (project, id, ended_at)
  VALUES (?1, ?2, ?3)
  ON CONFLICT (project, id) DO UPDATE SET ended_at = excluded.ended_at
  WHERE ended_at IS NULL OR excluded.ended_at > ended_at";

/// Record `begun`; the same start twice changes nothing.
pub(super) fn start(conn: &Connection, begun: &Begun) -> Result<(), Error> {
    let s = &begun.start;
    let args = params![begun.project, s.id, s.ide, s.cwd, s.ts];
    conn.execute(START, args)?;
    Ok(())
}

/// Record `ended`; the same end twice changes nothing.
pub(super) fn end(conn: &Connection, ended: &Ended) -> Result<(), Error> {
    conn.execute(END, params![ended.project, ended.id, ended.ts])?;
    Ok(())
}

const LIST: &str = "SELECT id, ide, cwd, coalesce(started_at, ended_at),
  ended_at FROM sessions WHERE project = ?1
  ORDER BY 4 DESC, id DESC LIMIT ?2";

fn session(r: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
    Ok(Session {
        id: r.get(0)?,
        ide: r.get(1)?,
        cwd: r.get(2)?,
        started_at: r.get(3)?,
        ended_at: r.get(4)?,
    })
}

impl Store {
    /// Record the start of a session of this project, as `session-start`
    /// sees it, from either mode (src/store T16): the earliest start
    /// seen wins, so the same call twice, or a resume, changes nothing.
    /// A lock outlasting the busy timeout spools it (V10).
    ///
    /// # Errors
    /// An unusable catalog or spool.
    pub fn session_started(
        &mut self,
        start: &SessionStart,
    ) -> Result<Written, Error> {
        let begun = Begun {
            project: self.project.clone(),
            start: start.clone(),
        };
        let item = Item::Begun(begun.clone());
        let stored = self.store(&item, |cx| self::start(cx.catalog, &begun))?;
        Ok(stored.map_or(Written::Spooled, |()| Written::Stored))
    }

    /// Record the end of the session `id` of this project at `ts`, from
    /// either mode (src/store T16): the latest end seen wins; an end
    /// before any start is kept. Spooled like a start (V10).
    ///
    /// # Errors
    /// An unusable catalog or spool.
    pub fn session_ended(
        &mut self,
        id: &str,
        ts: u64,
    ) -> Result<Written, Error> {
        let ended = Ended {
            project: self.project.clone(),
            id: id.to_owned(),
            ts,
        };
        let item = Item::Ended(ended.clone());
        let stored = self.store(&item, |cx| end(cx.catalog, &ended))?;
        Ok(stored.map_or(Written::Spooled, |()| Written::Stored))
    }

    /// Up to `limit` of this project's sessions, newest start first
    /// (src/store T16); one with only an end recorded starts at it.
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; a failed query.
    pub fn sessions(&mut self, limit: usize) -> Result<Vec<Session>, Error> {
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let mut stmt = self.reader()?.prepare(LIST)?;
        let rows = stmt.query_map(params![self.project, limit], session)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}
