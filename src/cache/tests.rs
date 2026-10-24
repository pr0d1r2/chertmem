//! Handle tests, and the scratch data dir every cache test opens.

use std::error::Error as StdError;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::{Cache, Env, Handle, Source};
use crate::config::{Config, load_from};
use crate::store::{Event, Filter, Mode, Store};

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

    pub(super) fn config(&self) -> Result<Config, Box<dyn StdError>> {
        config_at(&self.0)
    }

    pub(super) fn open(&self, mode: Mode) -> Result<Store, Box<dyn StdError>> {
        let root = self.0.join("project");
        Ok(Store::open(&self.config()?, &root, mode)?)
    }

    /// A cold handle on its own reading store, its warnings collected.
    pub(super) fn cold(&self, cfg: &Config) -> Result<Cold, Box<dyn StdError>> {
        let root = self.0.join("project");
        let store = Store::open(cfg, &root, Mode::Read)?;
        let (env, logs) = env();
        Ok(Cold {
            handle: Handle::cold(store, cfg, env),
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
    (Env { now, log }, logs)
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
