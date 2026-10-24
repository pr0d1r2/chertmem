//! JSONL import: each line of a `chert export` read back into its
//! observation and restored under its own id, ts, project and ref
//! (src/import V1, src/store `restore`).

#[cfg(test)]
mod tests;

use std::io::BufRead;

use super::Error;
use crate::store::Store;

/// Restore every line of `input`, an export, into `store`, which needs a
/// reading open; the number of lines read. A line whose row is already
/// stored adds nothing, so importing twice adds nothing (V1).
///
/// # Errors
/// [`Error::Line`] naming the first line that is not an export line or
/// that the store refuses, e.g. a ref another id holds.
pub fn jsonl(store: &mut Store, input: impl BufRead) -> Result<usize, Error> {
    let _ = (store, input);
    Ok(0)
}
