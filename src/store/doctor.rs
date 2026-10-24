//! `doctor`: rebuild `catalog.db` from the partition files, each of
//! which holds its own catalog row, so a deleted or stale catalog comes
//! back equal to the one before the crash (src/store T7, V6).

#[cfg(test)]
mod tests;

use super::{Error, Store};

impl Store {
    /// Rebuild the catalog from the files under `chunks/`.
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; an unreadable file.
    pub fn rebuild(&mut self) -> Result<(), Error> {
        self.reading() // RED stub
    }
}
