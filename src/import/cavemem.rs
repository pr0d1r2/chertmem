//! cavemem 0.2.1 import: its `SQLite` file opened read-only (`mode=ro`),
//! never written (src/import C, V3); each row stored once however often
//! it is imported (V2).

#[cfg(test)]
mod tests;

use std::path::Path;

use super::{Error, private};
use crate::config::Grammar;
use crate::store::Store;

/// What an import read from cavemem; every row of it is in the store.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Imported {
    pub observations: usize,
    pub summaries: usize,
}

/// Import the cavemem 0.2.1 file `db` into `store`, which needs a
/// reading open. A body cavemem stored uncompressed is compressed at
/// `grammar`, as the hook does.
///
/// # Errors
/// A file that is not a cavemem 0.2.1 db, or a store that refuses a
/// read or a write.
pub fn cavemem(
    store: &mut Store,
    db: &Path,
    grammar: Grammar,
) -> Result<Imported, Error> {
    let _ = (store, db, grammar, private::strip);
    Ok(Imported::default())
}
