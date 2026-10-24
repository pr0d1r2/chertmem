//! The cold tier's files: `<path>.zst`, 1 zstd frame of the partition
//! `VACUUM INTO` wrote, `<path>.dict` beside it when a dictionary was
//! trained, and the copy a read thaws to under `thawed/` (src/store T6).

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use super::Error;
use crate::config::Zstd;

/// `<root>/<path><end>`.
fn beside(root: &Path, path: &str, end: &str) -> PathBuf {
    let mut file = root.join(path).into_os_string();
    file.push(end);
    PathBuf::from(file)
}

/// The compressed partition.
pub(super) fn zst(root: &Path, path: &str) -> PathBuf {
    beside(root, path, ".zst")
}

/// Its dictionary, when 1 was trained.
pub(super) fn dict(root: &Path, path: &str) -> PathBuf {
    beside(root, path, ".dict")
}

/// Where a read decompresses it to.
pub(super) fn thawed(root: &Path, path: &str) -> PathBuf {
    root.join("thawed").join(path)
}

/// Compress `src` to the cold file of `path`.
pub(super) fn freeze(
    src: &Path,
    root: &Path,
    path: &str,
    zstd: Zstd,
) -> Result<(), Error> {
    let _ = (src, zstd, dict(root, path), thaw(root, path)?);
    Ok(()) // RED stub
}

/// The thawed copy of `path`, decompressed on first use.
pub(super) fn thaw(root: &Path, path: &str) -> Result<PathBuf, Error> {
    let out = thawed(root, path);
    std::fs::create_dir_all(root).map_err(|e| Error::io(root, &e))?;
    Ok(out) // RED stub
}

/// Remove every file of `path`, in every tier.
pub(super) fn remove(root: &Path, path: &str) -> Result<(), Error> {
    let _ = zst(root, path);
    std::fs::create_dir_all(root).map_err(|e| Error::io(root, &e)) // RED stub
}
