//! The cold tier's files: `<path>.zst`, 1 zstd frame of the partition
//! `VACUUM INTO` wrote, `<path>.dict` beside it when a dictionary was
//! trained, and the copy a read thaws to under `thawed/` (src/store T6).

#[cfg(test)]
mod tests;

use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};

use zstd::bulk::Compressor;
use zstd::stream::read::Decoder;

use super::Error;
use crate::config::Zstd;

/// The samples a dictionary learns from: `SQLite` pages.
const PAGE: usize = 4096;
/// The largest dictionary, and at most 1/8 of the file it serves.
const DICT_MAX: usize = 16 * 1024;
/// A hot file and its WAL companions.
const HOT: [&str; 3] = ["", "-wal", "-shm"];

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

/// Compress `src` to the cold file of `path`, with a dictionary trained
/// on its pages when `zstd.dictionary` is on and training succeeds. The
/// dictionary lands first: a `.zst` is whole once it exists.
pub(super) fn freeze(
    src: &Path,
    root: &Path,
    path: &str,
    zstd: Zstd,
) -> Result<(), Error> {
    let data = read(src)?.ok_or_else(|| missing(src))?;
    let trained = if zstd.dictionary { train(&data) } else { None };
    let out = zst(root, path);
    let packed = pack(&data, zstd.level, trained.as_deref());
    let packed = packed.map_err(|e| Error::io(&out, &e))?;
    if let Some(d) = &trained {
        write(&dict(root, path), d)?;
    }
    write(&out, &packed)
}

fn train(data: &[u8]) -> Option<Vec<u8>> {
    let sizes: Vec<usize> = data.chunks(PAGE).map(<[u8]>::len).collect();
    let max = (data.len() / 8).min(DICT_MAX);
    zstd::dict::from_continuous(data, &sizes, max).ok()
}

fn pack(
    data: &[u8],
    level: i32,
    dict: Option<&[u8]>,
) -> std::io::Result<Vec<u8>> {
    let mut c = match dict {
        Some(d) => Compressor::with_dictionary(level, d)?,
        None => Compressor::new(level)?,
    };
    c.compress(data)
}

/// The thawed copy of `path`, decompressed on first use; written whole
/// then renamed, so 2 readers at once never see half a file.
pub(super) fn thaw(root: &Path, path: &str) -> Result<PathBuf, Error> {
    let out = thawed(root, path);
    if out.is_file() {
        return Ok(out);
    }
    let src = zst(root, path);
    let packed = read(&src)?.ok_or_else(|| missing(&src))?;
    let dict = read(&dict(root, path))?;
    let data = unpack(&packed, dict.as_deref());
    let data = data.map_err(|e| Error::io(&src, &e))?;
    write(&out, &data)?;
    Ok(out)
}

fn unpack(packed: &[u8], dict: Option<&[u8]>) -> std::io::Result<Vec<u8>> {
    let mut data = Vec::new();
    match dict {
        Some(d) => Decoder::with_dictionary(packed, d)?.read_to_end(&mut data),
        None => Decoder::with_buffer(packed)?.read_to_end(&mut data),
    }?;
    Ok(data)
}

/// Remove the hot file of `path` and its WAL companions.
pub(super) fn remove_hot(root: &Path, path: &str) -> Result<(), Error> {
    HOT.iter()
        .try_for_each(|end| unlink(&beside(root, path, end)))
}

/// Remove every file of `path`, in every tier, and its thawed copy.
pub(super) fn remove(root: &Path, path: &str) -> Result<(), Error> {
    remove_hot(root, path)?;
    remove_hot(&root.join("thawed"), path)?;
    unlink(&zst(root, path))?;
    unlink(&dict(root, path))
}

/// Remove `file`; one already gone is fine.
pub(super) fn unlink(file: &Path) -> Result<(), Error> {
    match std::fs::remove_file(file) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(Error::io(file, &e)),
        _ => Ok(()),
    }
}

fn missing(file: &Path) -> Error {
    Error::Io {
        path: file.to_path_buf(),
        reason: "no such file".to_owned(),
    }
}

/// The bytes of `file`; `None` when it does not exist.
fn read(file: &Path) -> Result<Option<Vec<u8>>, Error> {
    match std::fs::read(file) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::io(file, &e)),
    }
}

/// Write `bytes` to a temp file beside `file`, then rename it into place.
fn write(file: &Path, bytes: &[u8]) -> Result<(), Error> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, &e))?;
    }
    let mut tmp = file.as_os_str().to_owned();
    tmp.push(format!(".{}.tmp", ulid::Ulid::generate()));
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes).map_err(|e| Error::io(&tmp, &e))?;
    std::fs::rename(&tmp, file).map_err(|e| Error::io(file, &e))
}
