//! Where a row lands: the project id, the partition's dims, its time
//! bucket of the row's own timestamp, its shard and the file under
//! `chunks/` (src/store C, V1, R1).

#[cfg(test)]
mod tests;

use std::path::Path;

use super::Error;
use super::calendar;
use crate::config::{Dimension, Partition, Span};

/// FNV-1a 64: the shard hash, the project id hash and the partition
/// checksum (src/store R1).
pub(super) const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

/// Continue an FNV-1a 64 hash over `bytes`.
pub(super) fn fnv1a(state: u64, bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(state, |h, &b| (h ^ u64::from(b)).wrapping_mul(FNV_PRIME))
}

/// The stable id of the project at `root`: its directory name, then the
/// FNV-1a 64 of its canonical path (src/store R1).
#[must_use]
pub fn project_id(root: &Path) -> String {
    let path = std::fs::canonicalize(root).unwrap_or_else(|_| root.into());
    let name = path.file_name().map(|n| slug(&n.to_string_lossy()));
    let name = name.filter(|n| !n.is_empty());
    let hash = fnv1a(FNV_OFFSET, path.as_os_str().as_encoded_bytes());
    format!("{}-{hash:016x}", name.as_deref().unwrap_or("root"))
}

/// Lowercase ASCII letters, digits, `-` and `_`; anything else `-`.
fn slug(text: &str) -> String {
    let keep = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    let chars = text.chars().take(32);
    chars
        .map(|c| if keep(c) { c.to_ascii_lowercase() } else { '-' })
        .collect()
}

/// What routes a row to its partition.
#[derive(Debug, Clone, Copy)]
pub(super) struct Key<'a> {
    pub project: &'a str,
    pub ide: &'a str,
    pub ts: u64,
    pub id: &'a str,
}

/// A row's partition: its file and the catalog's dims and bounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Place {
    /// Relative to the data dir.
    pub path: String,
    /// Empty when `project` is not a dim.
    pub project: String,
    /// Empty when `ide` is not a dim.
    pub ide: String,
    pub t0: u64,
    pub t1: u64,
    pub shard: Option<u32>,
}

impl Place {
    /// The `n`-th fresh partition of this bucket, `<stem>~<n>.db`, where
    /// rows land once the ones before are sealed or dropped (src/store
    /// V13); the 0th is this place itself.
    pub(super) fn nth(&self, n: u32) -> Self {
        let mut next = self.clone();
        if n > 0 {
            let stem = self.path.strip_suffix(".db").unwrap_or(&self.path);
            next.path = format!("{stem}~{n}.db");
        }
        next
    }
}

/// The current `partition.*`; it routes writes only, never reads
/// (src/store V4).
#[derive(Debug, Clone)]
pub(super) struct Plan {
    by: Vec<Dimension>,
    bucket: Span,
    shards: u32,
}

impl Plan {
    pub(super) fn new(p: &Partition) -> Self {
        Self {
            by: p.by.clone(),
            bucket: p.time_bucket,
            shards: p.shards,
        }
    }

    /// `partition.by` as written, e.g. `project,time`.
    pub(super) fn dims(&self) -> String {
        let names = self.by.iter().map(|d| match d {
            Dimension::Project => "project",
            Dimension::Time => "time",
            Dimension::Ide => "ide",
        });
        names.collect::<Vec<_>>().join(",")
    }

    /// The spec a partition is written under (src/store C).
    pub(super) fn spec(&self) -> String {
        let (dims, bucket) = (self.dims(), self.bucket);
        format!("by={dims};time_bucket={bucket};shards={}", self.shards)
    }

    /// The partition of `key`, bucketed by its own timestamp (V1).
    pub(super) fn place(&self, key: Key<'_>) -> Result<Place, Error> {
        let bad = Error::BadTime { ts: key.ts };
        let (t0, t1) = calendar::bounds(self.bucket, key.ts).ok_or(bad)?;
        let project = self.dim(Dimension::Project, key.project);
        let ide = self.dim(Dimension::Ide, &slug(key.ide));
        let shard = self.shard(key);
        let path = self.file(&self.dir(&project, &ide), t0, shard);
        Ok(Place {
            path,
            project,
            ide,
            t0,
            t1,
            shard,
        })
    }

    /// `chunks/<dir>/<bucket>[.<shard>].db` (src/store C).
    fn file(&self, dir: &str, t0: u64, shard: Option<u32>) -> String {
        let name = calendar::bucket_name(self.bucket, t0);
        let suffix = shard.map(|s| format!(".{s}")).unwrap_or_default();
        format!("chunks/{dir}/{name}{suffix}.db")
    }

    /// `value` when `dim` partitions (`_` for an empty one), else empty.
    fn dim(&self, dim: Dimension, value: &str) -> String {
        if !self.by.contains(&dim) {
            return String::new();
        }
        if value.is_empty() { "_" } else { value }.to_owned()
    }

    /// The non-time dims in `by` order, `+`-joined; `_` when none.
    fn dir(&self, project: &str, ide: &str) -> String {
        let parts: Vec<&str> = (self.by.iter())
            .filter_map(|d| match d {
                Dimension::Project => Some(project),
                Dimension::Ide => Some(ide),
                Dimension::Time => None,
            })
            .collect();
        if parts.is_empty() {
            "_".to_owned()
        } else {
            parts.join("+")
        }
    }

    /// hash(project, id) mod `shards`; `None` for 1 shard (src/store C).
    fn shard(&self, key: Key<'_>) -> Option<u32> {
        if self.shards < 2 {
            return None;
        }
        let h = fnv1a(fnv1a(FNV_OFFSET, key.project.as_bytes()), &[0]);
        let h = fnv1a(h, key.id.as_bytes());
        let shard = h.checked_rem(u64::from(self.shards))?;
        u32::try_from(shard).ok()
    }
}
