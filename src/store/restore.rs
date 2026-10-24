//! Restore: an exported observation stored back under its own id, ts,
//! project and issued ref (`src/import` V1, root V15).

#[cfg(test)]
mod tests;

use super::{Appended, Error, Observation, Store};

impl Store {
    /// Store `observation` under its own id, ts and ref.
    ///
    /// # Errors
    /// Not built yet.
    pub fn restore(
        &mut self,
        observation: &Observation,
    ) -> Result<Appended, Error> {
        let _ = (&self.root, observation);
        Err(Error::WriteOnly)
    }
}
