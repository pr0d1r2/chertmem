//! A payload the hook could not store, kept raw so it is never lost
//! (src/hook V2, root V2).
//!
//! The hook does not write the store's dir itself: it removes the
//! private spans (V1), then hands the bytes to [`Store::spool_raw`],
//! which writes them as given to `spool/unreadable/<ULID>.hook`, where
//! no replay reads or drops them (src/store T14, src/hook T8).

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use super::{Ctx, redact};
use crate::store::{Error, Mode, Store};

/// Keep `stdin` raw in the spool; the file kept, or why it could not be.
/// The store is opened write-only, which touches nothing before the
/// write, so it still opens when a reading open or an append failed;
/// only a missing data dir stops it ([`Error::NoDataDir`]).
pub(super) fn keep(ctx: &Ctx<'_>, stdin: &[u8]) -> Result<PathBuf, Error> {
    let bytes = redact::strip_bytes(stdin);
    Store::open(ctx.config, ctx.root, Mode::Write)?.spool_raw(&bytes)
}
