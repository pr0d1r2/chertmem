//! Sessions: 1 catalog row per project and session id, as a session
//! spans buckets; the hook records its start and end, `src/mcp` lists
//! them (src/store T16).

#[cfg(test)]
mod tests;

use super::{Error, Session, SessionStart, Store, Written};

impl Store {
    /// Record the start of a session of this project (src/store T16).
    ///
    /// # Errors
    /// Not built yet.
    pub fn session_started(
        &mut self,
        start: &SessionStart,
    ) -> Result<Written, Error> {
        let _ = (&self.root, start);
        Err(Error::WriteOnly)
    }

    /// Record the end of the session `id` of this project (src/store
    /// T16).
    ///
    /// # Errors
    /// Not built yet.
    pub fn session_ended(
        &mut self,
        id: &str,
        ts: u64,
    ) -> Result<Written, Error> {
        let _ = (&self.root, id, ts);
        Err(Error::WriteOnly)
    }

    /// This project's sessions, newest first (src/store T16).
    ///
    /// # Errors
    /// Not built yet.
    pub fn sessions(&mut self, limit: usize) -> Result<Vec<Session>, Error> {
        let _ = (&self.root, limit);
        Ok(Vec::new())
    }
}
