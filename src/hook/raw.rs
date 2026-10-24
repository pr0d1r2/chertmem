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

use std::fs;
use std::path::{Path, PathBuf};

use super::redact;
use crate::config::Config;

/// Keep `stdin` raw in the spool; the file kept, or why it could not be.
pub(super) fn keep(config: &Config, stdin: &[u8]) -> Result<PathBuf, String> {
    let data = config.storage.data_dir.as_deref().ok_or("no data dir")?;
    let dir = data.join("spool").join("unreadable");
    fs::create_dir_all(&dir).map_err(|e| failed(&dir, &e))?;
    let name = ulid::Ulid::generate().to_string();
    let tmp = dir.join(format!(".{name}.tmp"));
    let bytes = redact::strip_bytes(stdin);
    fs::write(&tmp, bytes).map_err(|e| failed(&tmp, &e))?;
    let done = dir.join(format!("{name}.hook"));
    fs::rename(&tmp, &done).map_err(|e| failed(&done, &e))?;
    Ok(done)
}

fn failed(path: &Path, e: &std::io::Error) -> String {
    format!("{}: {e}", path.display())
}
