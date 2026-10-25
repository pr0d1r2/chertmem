//! Handle tests, and the scratch data dir every cache test opens.

use std::any::Any;
use std::collections::BTreeMap;
use std::error::Error as StdError;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use rusqlite::{Connection, params};

use super::index::Rowless;
use super::{Cache, Env, Handle, Index, Key, Rows, Source};
use crate::config::{Config, load_from};
use crate::store::{Event, Filter, Mode, Observation, Ref, Store};

pub(super) type Test = Result<(), Box<dyn StdError>>;

pub(super) const DAY: u64 = 86_400_000;
pub(super) const HOUR: u64 = 3_600_000;
/// The injected clock: day 1000 after the epoch.
pub(super) const NOW: u64 = 1000 * DAY;
/// The default `cache.window` (28d) back from [`NOW`].
pub(super) const START: u64 = NOW - 28 * DAY;

pub(super) fn now() -> u64 {
    NOW
}

/// The config of the scratch dir `base`: no user file, data in
/// `base/data`.
pub(super) fn config_at(base: &Path) -> Result<Config, Box<dyn StdError>> {
    let mut cfg = load_from(None, &base.join("project"))?.config;
    cfg.storage.data_dir = Some(base.join("data"));
    Ok(cfg)
}

/// A scratch dir holding `project/` and the data dir `data/`, removed on
/// drop.
pub(super) struct Scratch(PathBuf);

impl Scratch {
    pub(super) fn new(name: &str) -> Result<Self, Box<dyn StdError>> {
        let pid = std::process::id();
        let dir =
            std::env::temp_dir().join(format!("chert-cache-{pid}-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("project"))?;
        Ok(Self(dir))
    }

    pub(super) fn base(&self) -> &Path {
        &self.0
    }

    /// A raw connection to `path` under the data dir.
    pub(super) fn sql(
        &self,
        path: &str,
    ) -> Result<Connection, Box<dyn StdError>> {
        Ok(Connection::open(self.0.join("data").join(path))?)
    }

    pub(super) fn config(&self) -> Result<Config, Box<dyn StdError>> {
        config_at(&self.0)
    }

    pub(super) fn open(&self, mode: Mode) -> Result<Store, Box<dyn StdError>> {
        let root = self.0.join("project");
        Ok(Store::open(&self.config()?, &root, mode)?)
    }

    /// A cold handle on its own reading store, its warnings collected.
    pub(super) fn cold(&self, cfg: &Config) -> Result<Cold, Box<dyn StdError>> {
        self.indexed(cfg, Box::new(Rowless))
    }

    /// [`Scratch::cold`] feeding `index`.
    pub(super) fn indexed(
        &self,
        cfg: &Config,
        index: Box<dyn Index>,
    ) -> Result<Cold, Box<dyn StdError>> {
        let root = self.0.join("project");
        let store = Store::open(cfg, &root, Mode::Read)?;
        let (env, logs) = env();
        Ok(Cold {
            handle: Handle::cold(store, cfg, env, index),
            logs,
        })
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) type Logs = Arc<Mutex<Vec<String>>>;

/// A handle not yet warmed and the warnings it logged.
pub(super) struct Cold {
    pub(super) handle: Handle,
    pub(super) logs: Logs,
}

/// The injected clock and a log sink collecting every warning.
pub(super) fn env() -> (Env, Logs) {
    let logs = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&logs);
    let log = Arc::new(move |text: &str| {
        if let Ok(mut all) = sink.lock() {
            all.push(text.to_owned());
        }
    });
    let env = Env {
        now,
        memory: plenty,
        log,
    };
    (env, logs)
}

/// A memory figure no test config comes near: 1 TiB.
fn plenty() -> Option<u64> {
    1_u64.checked_shl(40)
}

/// Every warning logged so far.
pub(super) fn logged(logs: &Logs) -> Result<Vec<String>, Box<dyn StdError>> {
    Ok(logs.lock().map_err(|_| "poisoned")?.clone())
}

pub(super) fn event(ts: u64, body: &str) -> Event {
    Event {
        ts,
        session: "session-1".into(),
        kind: "post-tool-use".into(),
        ide: "claude-code".into(),
        identity: None,
        body: body.into(),
    }
}

/// The project's rows from `since` on.
pub(super) fn from(store: &Store, since: u64) -> Filter {
    Filter {
        project: Some(store.project().to_owned()),
        since: Some(since),
        until: None,
    }
}

/// `q` through `handle` equals `q` through `truth` alone (V1); which
/// side answered.
pub(super) fn same(
    handle: &Handle,
    truth: &mut Store,
    q: &Filter,
) -> Result<Source, Box<dyn StdError>> {
    let (rows, source) = handle.answer(q)?;
    assert_eq!(rows, truth.observations(q)?, "{q:?} via {source:?}");
    Ok(source)
}

/// Wait up to 10 s for `handle` to be warm.
pub(super) fn wait_warm(handle: &Handle) -> Test {
    let started = Instant::now();
    while !handle.is_warm()? {
        if started.elapsed() > Duration::from_secs(10) {
            return Err("the cache never warmed".into());
        }
        thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

/// Rows in 2 buckets of the window and 1 before it.
pub(super) fn seed(s: &Scratch) -> Test {
    let mut hook = s.open(Mode::Write)?;
    for (ts, body) in [
        (START - DAY, "before the window"),
        (NOW - 20 * DAY, "early"),
        (NOW - HOUR, "late"),
        (NOW - 2 * HOUR, "later appended, earlier ts"),
    ] {
        hook.append(&event(ts, body))?;
    }
    Ok(())
}

#[test]
fn queries_go_to_the_store_until_warm() -> Test {
    let s = Scratch::new("cold")?;
    seed(&s)?;
    let cold = s.cold(&s.config()?)?;
    let mut truth = s.open(Mode::Read)?;
    let q = from(&truth, START);
    assert_eq!(same(&cold.handle, &mut truth, &q)?, Source::Store);
    assert_eq!(cold.handle.query(&q)?.len(), 3);
    Ok(())
}

#[test]
fn a_warm_window_answers_from_memory_as_the_store_does() -> Test {
    let s = Scratch::new("warm")?;
    seed(&s)?;
    let cold = s.cold(&s.config()?)?;
    cold.handle.warm()?;
    let mut truth = s.open(Mode::Read)?;
    for since in [START, NOW - 21 * DAY, NOW - HOUR] {
        let mut q = from(&truth, since);
        q.until = Some(NOW - HOUR);
        let up_to = same(&cold.handle, &mut truth, &q)?;
        q.until = None;
        let open = same(&cold.handle, &mut truth, &q)?;
        assert_eq!([up_to, open], [Source::Cache; 2], "{since}");
    }
    assert!(logged(&cold.logs)?.is_empty());
    Ok(())
}

#[test]
fn a_query_reaching_outside_the_window_goes_to_the_store() -> Test {
    let s = Scratch::new("outside")?;
    seed(&s)?;
    let cold = s.cold(&s.config()?)?;
    cold.handle.warm()?;
    let mut truth = s.open(Mode::Read)?;
    let mut other = from(&truth, START);
    other.project = Some("other".into());
    let all = Filter::default();
    for q in [from(&truth, START - 1), all, other] {
        assert_eq!(same(&cold.handle, &mut truth, &q)?, Source::Store);
    }
    assert_eq!(cold.handle.query(&from(&truth, 0))?.len(), 4);
    Ok(())
}

#[test]
fn start_warms_in_the_background() -> Test {
    let s = Scratch::new("start")?;
    seed(&s)?;
    let cfg = s.config()?;
    let (env, logs) = env();
    let handle = Cache::start_with(s.open(Mode::Read)?, &cfg, env);
    wait_warm(&handle)?;
    let mut truth = s.open(Mode::Read)?;
    let q = from(&truth, START);
    assert_eq!(same(&handle, &mut truth, &q)?, Source::Cache);
    assert!(logged(&logs)?.is_empty());
    Ok(())
}

#[test]
fn start_runs_on_the_system_clock() -> Test {
    let s = Scratch::new("system")?;
    s.open(Mode::Write)?.append(&event(NOW, "long ago"))?;
    let env = Env::system();
    assert!((env.now)() > NOW);
    (env.log)("chert: a test warning on stderr (ignore it)");
    let handle = Cache::start(s.open(Mode::Read)?, &s.config()?);
    wait_warm(&handle)?;
    let mut truth = s.open(Mode::Read)?;
    let q = from(&truth, 0);
    assert_eq!(same(&handle, &mut truth, &q)?, Source::Store);
    Ok(())
}

/// What a delete, redaction or rewrite does once `src/store` has its
/// own (D15): change rows in place, then bump `generation`. Raw SQL
/// here only, until the store's drop lands.
pub(super) fn rewrite_elsewhere(s: &Scratch, store: &Store) -> Test {
    for p in store.partitions(&from(store, 0))? {
        let part = s.sql(&p.path)?;
        part.execute("DELETE FROM observations WHERE body = 'late'", [])?;
        let redact = "UPDATE observations SET body = 'gone' WHERE body = ?1";
        part.execute(redact, ["early"])?;
    }
    let bump = "UPDATE meta SET value = value + 1 WHERE key = 'generation'";
    s.sql("catalog.db")?.execute(bump, [])?;
    Ok(())
}

#[test]
fn a_new_generation_reloads_the_window() -> Test {
    let s = Scratch::new("generation")?;
    seed(&s)?;
    let cold = s.cold(&s.config()?)?;
    cold.handle.warm()?;
    let mut truth = s.open(Mode::Read)?;
    let q = from(&truth, START);
    rewrite_elsewhere(&s, &truth)?;
    assert_eq!(same(&cold.handle, &mut truth, &q)?, Source::Store);
    wait_warm(&cold.handle)?;
    assert_eq!(same(&cold.handle, &mut truth, &q)?, Source::Cache);
    let rows = cold.handle.query(&q)?;
    let bodies: Vec<&str> = rows.iter().map(|o| o.body.as_str()).collect();
    assert_eq!(bodies, ["gone", "later appended, earlier ts"]);
    Ok(())
}

const BROKEN: &str = "INSERT INTO partitions
  (path, dims, project, ide, t0, t1, checksum, spec)
  VALUES (?1, 'project,time', ?2, '', ?3, ?4, '0', '')";

#[test]
fn a_failed_refresh_drops_the_window_and_logs_why() -> Test {
    let s = Scratch::new("broken")?;
    seed(&s)?;
    let cold = s.cold(&s.config()?)?;
    cold.handle.warm()?;
    let truth = s.open(Mode::Read)?;
    // A catalog row naming a file that is not SQLite.
    let path = format!("chunks/{}/broken.db", truth.project());
    fs::write(s.base().join("data").join(&path), "not a database")?;
    let args = params![path, truth.project(), NOW - DAY, NOW + DAY];
    s.sql("catalog.db")?.execute(BROKEN, args)?;
    assert!(cold.handle.query(&from(&truth, START)).is_err());
    assert!(!cold.handle.is_warm()?);
    assert!(!logged(&cold.logs)?.is_empty());
    Ok(())
}

/// A row of `body` at `ts`, for an index fed by hand.
pub(super) fn row(
    ts: u64,
    body: &str,
) -> Result<Observation, Box<dyn StdError>> {
    Ok(Observation {
        reference: Ref::parse("chert:0000000A")?,
        id: format!("{ts:026}"),
        project: "p".into(),
        ts,
        session: "session-1".into(),
        ide: "claude-code".into(),
        kind: "post-tool-use".into(),
        identity: None,
        body: body.into(),
    })
}

/// An index recording what the window fed it, `per_row` bytes a row.
pub(super) struct Recorder {
    pub(super) rows: BTreeMap<Key, String>,
    pub(super) replaced: Vec<String>,
    pub(super) resets: u64,
    per_row: u64,
}

impl Recorder {
    pub(super) fn new(per_row: u64) -> Self {
        Self {
            rows: BTreeMap::new(),
            replaced: Vec::new(),
            resets: 0,
            per_row,
        }
    }
}

impl Index for Recorder {
    fn reset(&mut self) {
        self.rows.clear();
        self.resets = self.resets.saturating_add(1);
    }

    fn insert(
        &mut self,
        key: &Key,
        row: &Observation,
        replaced: Option<&Observation>,
    ) {
        if let Some(old) = replaced {
            self.replaced.push(old.body.clone());
        }
        self.rows.insert(key.clone(), row.body.clone());
    }

    fn bytes(&self) -> u64 {
        let n = u64::try_from(self.rows.len()).unwrap_or(u64::MAX);
        n.saturating_mul(self.per_row)
    }
}

/// What [`Handle::with_index`] lent: the rows cloned, and the
/// recorder's rows and resets.
pub(super) struct Lent {
    pub(super) rows: Vec<Observation>,
    pub(super) index: BTreeMap<Key, String>,
    pub(super) resets: u64,
}

fn copy(rows: Rows<'_>, index: &dyn Index) -> Option<Lent> {
    let any: &dyn Any = index;
    let recorder = any.downcast_ref::<Recorder>()?;
    Some(Lent {
        rows: rows.to_vec(),
        index: recorder.rows.clone(),
        resets: recorder.resets,
    })
}

/// `q` through [`Handle::with_index`] on a [`Recorder`]; `None`: the
/// window declined it.
pub(super) fn lend(
    handle: &Handle,
    q: &Filter,
) -> Result<Option<Lent>, Box<dyn StdError>> {
    let lent = handle.with_index(q, copy)?;
    Ok(lent.map(|l| l.ok_or("not a recorder")).transpose()?)
}

/// A cold handle on `s` fed a [`Recorder`] of `per_row` bytes a row.
pub(super) fn recorded(
    s: &Scratch,
    cfg: &Config,
    per_row: u64,
) -> Result<Cold, Box<dyn StdError>> {
    s.indexed(cfg, Box::new(Recorder::new(per_row)))
}

/// The seeded scratch `name` and a handle on it fed a [`Recorder`],
/// warmed.
fn warmed(name: &str) -> Result<(Scratch, Cold), Box<dyn StdError>> {
    let s = Scratch::new(name)?;
    seed(&s)?;
    let cold = recorded(&s, &s.config()?, 1)?;
    cold.handle.warm()?;
    Ok((s, cold))
}

#[test]
fn with_index_lends_what_query_answers_and_the_index_fed_by_it() -> Test {
    let (s, cold) = warmed("lend")?;
    let mut truth = s.open(Mode::Read)?;
    let all = cold.handle.query(&from(&truth, START))?;
    let window: Vec<&String> = all.iter().map(|o| &o.body).collect();
    let mut bounded = from(&truth, START);
    bounded.until = Some(NOW - HOUR);
    for q in [from(&truth, START), bounded, from(&truth, NOW - HOUR)] {
        let lent = lend(&cold.handle, &q)?.ok_or("declined")?;
        assert_eq!(lent.rows, truth.observations(&q)?, "{q:?}");
        let bodies: Vec<&String> = lent.index.values().collect();
        assert_eq!((&bodies, lent.resets), (&window, 1), "{q:?}");
    }
    Ok(())
}

#[test]
fn with_index_declines_whatever_query_reads_from_the_store() -> Test {
    let s = Scratch::new("lend-decline")?;
    seed(&s)?;
    let cold = recorded(&s, &s.config()?, 1)?;
    let truth = s.open(Mode::Read)?;
    assert!(lend(&cold.handle, &from(&truth, START))?.is_none());
    cold.handle.warm()?;
    let mut other = from(&truth, START);
    other.project = Some("other".into());
    for q in [from(&truth, START - 1), Filter::default(), other] {
        assert!(lend(&cold.handle, &q)?.is_none(), "{q:?}");
    }
    Ok(())
}

/// The bodies at every key the recorder holds that `q`'s lent rows
/// find.
fn found(
    handle: &Handle,
    q: &Filter,
) -> Result<Vec<String>, Box<dyn StdError>> {
    let bodies = handle.with_index(q, |rows, index| {
        let any: &dyn Any = index;
        let keys = any.downcast_ref::<Recorder>().map(|r| r.rows.keys());
        let hits = keys.into_iter().flatten().filter_map(|k| rows.get(k));
        hits.map(|o| o.body.clone()).collect()
    })?;
    Ok(bodies.ok_or("declined")?)
}

#[test]
fn lent_rows_find_a_key_inside_the_filter_only() -> Test {
    let (s, cold) = warmed("lend-get")?;
    let truth = s.open(Mode::Read)?;
    let mut q = from(&truth, NOW - 3 * HOUR);
    q.until = Some(NOW - HOUR);
    assert_eq!(found(&cold.handle, &q)?, ["later appended, earlier ts"]);
    q.until = None;
    assert_eq!(found(&cold.handle, &q)?.len(), 2);
    Ok(())
}

/// Append 1 row elsewhere; the next lend of `q` has it indexed.
fn tail_one(s: &Scratch, handle: &Handle, q: &Filter) -> Test {
    s.open(Mode::Write)?
        .append(&event(NOW - 5 * HOUR, "tailed"))?;
    let tailed = lend(handle, q)?.ok_or("declined")?;
    assert!(tailed.index.values().any(|b| b == "tailed"));
    assert_eq!(tailed.index.len(), 4);
    Ok(())
}

#[test]
fn the_index_follows_tails_and_resets_on_a_reload() -> Test {
    let (s, cold) = warmed("lend-follow")?;
    let mut truth = s.open(Mode::Read)?;
    let q = from(&truth, START);
    tail_one(&s, &cold.handle, &q)?;
    rewrite_elsewhere(&s, &truth)?;
    assert!(lend(&cold.handle, &q)?.is_none());
    wait_warm(&cold.handle)?;
    let reloaded = lend(&cold.handle, &q)?.ok_or("declined")?;
    assert_eq!(reloaded.rows, truth.observations(&q)?);
    let bodies: Vec<&str> =
        reloaded.index.values().map(String::as_str).collect();
    assert_eq!(bodies, ["gone", "tailed", "later appended, earlier ts"]);
    assert_eq!(reloaded.resets, 2);
    Ok(())
}

/// 2 loads begun on `s`: the older one keeps nothing, the newer one
/// warms rows and index alike.
fn superseded(s: &Scratch) -> Test {
    let cold = recorded(s, &s.config()?, 1)?;
    let old = cold.handle.begin()?.ok_or("off")?;
    let new = cold.handle.begin()?.ok_or("off")?;
    cold.handle.load(old)?;
    assert!(!cold.handle.is_warm()?);
    cold.handle.load(new)?;
    assert!(cold.handle.is_warm()?);
    let truth = s.open(Mode::Read)?;
    let lent = lend(&cold.handle, &from(&truth, START))?.ok_or("declined")?;
    assert_eq!(lent.index.len(), lent.rows.len());
    Ok(())
}

#[test]
fn a_load_superseded_by_a_newer_begin_keeps_nothing() -> Test {
    // No partition: it stops after its last step, not at 1.
    superseded(&Scratch::new("superseded-empty")?)?;
    let s = Scratch::new("superseded")?;
    seed(&s)?;
    superseded(&s)
}
