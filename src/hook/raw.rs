//! A payload the hook could not store, kept raw so it is never lost
//! (src/hook V2, root V2).

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use crate::config::Config;

/// Keep `stdin` raw in the spool; the file kept, or why it could not be.
pub(super) fn keep(config: &Config, stdin: &[u8]) -> Result<PathBuf, String> {
    let _ = (config, stdin);
    Err(String::new())
}
