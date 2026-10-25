//! `chert config set`: 1 key written into 1 `chert.toml`, every other
//! byte of the file kept (src/config T6, V3, V4).

#[cfg(test)]
mod tests;

use std::path::Path;

use super::Error;

/// Set `key` to `value_text` in `file`; a stub that writes nothing.
///
/// # Errors
/// None yet.
pub fn set(
    file: &Path,
    key: &str,
    value_text: &str,
) -> Result<Option<&'static str>, Error> {
    let _ = (file, key, value_text);
    Ok(None)
}
