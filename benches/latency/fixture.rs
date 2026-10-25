//! Seeded fixtures (benches C, T2): the rows of a [`Spec`] written into
//! a data dir through `Store`, cached under `target/bench-fixtures/`
//! and keyed by [`GENERATOR`], so a 10^6-row fixture is built once.

#[cfg(test)]
mod tests;

mod rows;

use std::error::Error as StdError;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use chertmem::config::{Config, load_from};
use chertmem::grammar::{Intensity, compress};
use chertmem::store::{Mode, Store};

pub(crate) use rows::{Row, Rows};

/// What every bench and fixture call returns.
pub(crate) type Res<T> = Result<T, Box<dyn StdError>>;

/// The generator's version: a change to the rows changes it, so a
/// cached fixture of an older generator is built again.
pub(crate) const GENERATOR: &str = "fixture-1";
/// 1 `time_bucket` of the default `7d`, which partitions are cut by.
pub(crate) const WEEK: u64 = 604_800_000;
/// The fixture's newest instant, on a bucket edge: week 2930 since the
/// epoch, 2026-02-26. The benches' clock stands here.
pub(crate) const END: u64 = 1_772_064_000_000;

/// 1 fixture: `size` observations over `parts` weekly partitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Spec {
    pub(crate) seed: u64,
    pub(crate) size: u64,
    pub(crate) parts: u64,
}

impl Spec {
    pub(crate) const fn new(seed: u64, size: u64, parts: u64) -> Self {
        Self { seed, size, parts }
    }

    /// `1k-p52`: the size, then the partitions.
    pub(crate) fn label(&self) -> String {
        format!("{}-p{}", short(self.size), self.parts)
    }

    /// The cache dir's name: `<seed>-<size>-<parts>`.
    pub(crate) fn key(&self) -> String {
        format!("{}-{}-{}", self.seed, self.size, self.parts)
    }

    /// The time of event `i`: spread evenly over the last `parts`
    /// weeks before [`END`], so each week gets a row when `size` is at
    /// least `parts`.
    pub(crate) fn ts(&self, i: u64) -> u64 {
        let span = WEEK;
        let step = i.saturating_mul(span).checked_div(self.size);
        END.saturating_sub(span).saturating_add(step.unwrap_or(0))
    }
}

/// `n` as `1M`, `10k` or `200`: the largest whole unit.
fn short(n: u64) -> String {
    for (unit, size) in [("M", 1_000_000), ("k", 1000)] {
        if n >= size && n.checked_rem(size) == Some(0) {
            let whole = n.checked_div(size).unwrap_or_default();
            return format!("{whole}{unit}");
        }
    }
    n.to_string()
}

/// FNV-1a 64 over every row's `Debug` text: the fixture's bytes (V2).
pub(crate) fn digest(rows: impl Iterator<Item = Row>) -> u64 {
    let mut hash = Fnv::default();
    rows.for_each(|row| hash.row(&row));
    let _ = hash.0;
    Fnv::default().0
}

struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv {
    fn row(&mut self, row: &Row) {
        for b in format!("{row:?}\n").bytes() {
            self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
    }
}

/// A built fixture: `project/`, the project root, and `data/`.
#[derive(Debug, Clone)]
pub(crate) struct Fixture {
    pub(crate) dir: PathBuf,
    pub(crate) spec: Spec,
}

impl Fixture {
    pub(crate) fn root(&self) -> PathBuf {
        self.dir.join("project")
    }

    pub(crate) fn data(&self) -> PathBuf {
        self.dir.join("data")
    }

    /// The defaults, no user file, data in `data/`.
    pub(crate) fn config(&self) -> Res<Config> {
        let mut cfg = load_from(None, &self.root())?.config;
        cfg.storage.data_dir = Some(self.data());
        Ok(cfg)
    }

    /// A reading store on the fixture as cached.
    pub(crate) fn store(&self) -> Res<Store> {
        Ok(Store::open(&self.config()?, &self.root(), Mode::Read)?)
    }

    fn stamp(&self) -> PathBuf {
        self.dir.join("stamp")
    }

    /// The stamp's head: generator and key; the digest follows.
    fn head(&self) -> String {
        format!("{GENERATOR} {}", self.spec.key())
    }
}

/// Where fixtures are cached: `$CARGO_TARGET_DIR`, else `target/`.
pub(crate) fn base() -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("target"),
        PathBuf::from,
    );
    target.join("bench-fixtures")
}

/// The fixture of `spec` under `base`: built when its stamp is missing
/// or of another generator, the stamp written last.
pub(crate) fn cached_in(base: &Path, spec: Spec) -> Res<Fixture> {
    let fx = Fixture {
        dir: base.join(spec.key()),
        spec,
    };
    let stamp = fs::read_to_string(fx.stamp()).unwrap_or_default();
    if !stamp.starts_with(&format!("{} ", fx.head())) {
        rebuild(&fx)?;
    }
    Ok(fx)
}

fn rebuild(fx: &Fixture) -> Res<()> {
    match fs::remove_dir_all(&fx.dir) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e.into()),
        _ => fs::create_dir_all(fx.root())?,
    }
    let start = Instant::now();
    build(fx)?;
    let took = start.elapsed();
    eprintln!("fixture {} built in {took:.1?}", fx.spec.label());
    let digest = digest(Rows::new(fx.spec));
    let stamp = format!("{} {digest:016x}\n", fx.head());
    Ok(fs::write(fx.stamp(), stamp)?)
}

/// Write every row of the fixture's spec.
fn build(fx: &Fixture) -> Res<()> {
    let cfg = fx.config()?;
    let mut store = Store::open(&cfg, &fx.root(), Mode::Read)?;
    let grammar = cfg.compression.grammar.intensity();
    for row in Rows::new(fx.spec) {
        apply(&mut store, row, grammar)?;
    }
    Ok(())
}

/// 1 row as the hooks write it, bodies compressed (`src/hook` V1).
fn apply(store: &mut Store, row: Row, grammar: Intensity) -> Res<()> {
    match row {
        Row::Begin(start) => {
            store.session_started(&start)?;
        }
        Row::Event(mut event) => {
            event.body = compress(&event.body, grammar);
            drop(store.append(&event)?);
        }
        Row::Summary(mut summary) => {
            store.session_ended(&summary.session, summary.ts)?;
            summary.body = compress(&summary.body, grammar);
            store.append_summary(&summary)?;
        }
    }
    Ok(())
}

/// A temp dir of this process, removed on drop.
#[derive(Debug)]
pub(crate) struct Scratch(PathBuf);

impl Scratch {
    /// A fresh, empty temp dir named after `name`.
    pub(crate) fn new(name: &str) -> Res<Self> {
        let pid = std::process::id();
        let dir =
            std::env::temp_dir().join(format!("chert-bench-{pid}-{name}"));
        drop(fs::remove_dir_all(&dir));
        fs::create_dir_all(&dir)?;
        Ok(Self(dir))
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }

    /// A writable copy of `fx`'s data dir and the config naming it: the
    /// hook suites append, and the cached fixture stays as built.
    pub(crate) fn of(fx: &Fixture, name: &str) -> Res<(Self, Config)> {
        let copy = Self::new(name)?;
        copy_tree(&fx.data(), copy.path())?;
        let mut cfg = fx.config()?;
        cfg.storage.data_dir = Some(copy.path().to_path_buf());
        Ok((copy, cfg))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.0));
    }
}

fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
