//! Repartition: rewrite every partition written under another
//! `partition.*` spec to the current one, each row under its own id and
//! ref, so every ref keeps resolving to its own row (src/store T6, V5).

#[cfg(test)]
mod tests;

use super::{Error, Store};

impl Store {
    /// Rewrite every partition written under another `partition.*`
    /// spec to the current one; refs keep resolving (src/store V5).
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; a sealed target.
    pub fn repartition(&mut self) -> Result<(), Error> {
        self.reading() // RED stub
    }
}
