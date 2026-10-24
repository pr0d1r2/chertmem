//! A payload the hook could not store, kept raw so it is never lost
//! (src/hook V2, root V2).
//!
//! It lands in `spool/unreadable/<ULID>.hook` under the data dir, where
//! `src/store` sets aside spool files it cannot read: no replay reads or
//! drops it, so it waits for a person or a later importer. Written whole
//! then renamed into place, as the store's spool files are; its private
//! spans are removed first (V1).

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use super::Ctx;
use crate::store::Error;

/// Keep `stdin` raw in the spool; the file kept, or why it could not be.
/// A stub that refuses until the store keeps it (src/hook T8).
pub(super) fn keep(_ctx: &Ctx<'_>, _stdin: &[u8]) -> Result<PathBuf, Error> {
    Err(Error::NoDataDir)
}
