//! The recent `cache.window` of the current project in memory inside
//! `chert mcp` (src/cache G, D14): warmed on a background thread, the
//! store answering until then, fitted to `cache.max_bytes` (D16). The
//! store stays the truth (root C).

#[cfg(test)]
mod tests;

mod error;
mod fit;
mod index;
mod memory;
mod window;

use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

pub use error::Error;
pub use fit::Plan;
pub use index::{Index, Key};
pub use window::Rows;

use crate::config::{self, Config, Overflow};
use crate::store::{Filter, Observation, Store};
use fit::Fit;
use index::Rowless;
use window::{Fresh, Window};

/// Where warnings go: the MCP log, `status`, `doctor`; never into a
/// query result (src/cache V4, root V11).
pub type Log = Arc<dyn Fn(&str) + Send + Sync>;

/// What a cache takes from its host; injected so tests own the clock,
/// the memory figure and the log.
#[derive(Clone)]
pub struct Env {
    /// Milliseconds since the Unix epoch.
    pub now: fn() -> u64,
    /// Bytes of memory available; `None` when unknown (src/cache R2).
    pub memory: fn() -> Option<u64>,
    pub log: Log,
}

impl Env {
    /// The system clock and memory; warnings on stderr, which `chert
    /// mcp` keeps apart from its stdio protocol.
    #[must_use]
    pub fn system() -> Self {
        Self {
            now: clock,
            memory: memory::available,
            log: Arc::new(|text: &str| eprintln!("{text}")),
        }
    }
}

fn clock() -> u64 {
    let since = SystemTime::now().duration_since(UNIX_EPOCH);
    let millis = since.unwrap_or_default().as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}

/// The cache of 1 project's recent window (src/cache I).
pub struct Cache;

impl Cache {
    /// Check memory and warm `store`'s window on a background thread;
    /// until it is warm, queries go to the store (src/cache C).
    #[must_use]
    pub fn start(store: Store, cfg: &Config) -> Handle {
        Self::start_with(store, cfg, Env::system())
    }

    /// [`Cache::start`] with the clock, memory figure and log given.
    #[must_use]
    pub fn start_with(store: Store, cfg: &Config, env: Env) -> Handle {
        Self::start_indexed(store, cfg, env, Box::new(Rowless))
    }

    /// [`Cache::start_with`] feeding `index` every row of the window
    /// (T6), for [`Handle::with_index`].
    #[must_use]
    pub fn start_indexed(
        store: Store,
        cfg: &Config,
        env: Env,
        index: Box<dyn Index>,
    ) -> Handle {
        let handle = Handle::cold(store, cfg, env, index);
        handle.spawn(true);
        handle
    }

    /// The plan of `store`'s window on the system clock (`chert cache
    /// plan`): no cache started, no thread, no row loaded.
    ///
    /// # Errors
    /// A failed catalog read.
    pub fn plan(store: &Store, cfg: &Config) -> Result<Plan, Error> {
        Self::plan_at(store, cfg, clock())
    }

    /// [`Cache::plan`] for the window ending at `now`.
    ///
    /// # Errors
    /// A failed catalog read.
    pub fn plan_at(
        store: &Store,
        cfg: &Config,
        now: u64,
    ) -> Result<Plan, Error> {
        Ok(fit_at(store, &cfg.cache, now)?.plan)
    }
}

/// The window of `cfg` ending at `now` fitted from `store`'s catalog
/// stats (D16): the 1 estimate [`Cache::plan`] and a handle share.
fn fit_at(store: &Store, cfg: &config::Cache, now: u64) -> Result<Fit, Error> {
    let start = now.saturating_sub(fit::window_ms(cfg.window));
    let filter = Filter {
        project: Some(store.project().to_owned()),
        since: Some(start),
        until: None,
    };
    let parts = store.partitions(&filter)?;
    Ok(fit::fit(&parts, cfg, now))
}

/// Which side answered a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Cache,
    Store,
}

enum State {
    /// Not loaded: the store answers.
    Cold,
    Warm(Window),
    /// `on_overflow = "fail"` over the limit: disk only (D16).
    Off,
}

/// A window being loaded: its fit, and the load it belongs to.
struct Load {
    window: Window,
    fit: Fit,
    epoch: u64,
}

struct Inner {
    store: Store,
    cfg: config::Cache,
    state: State,
    /// Fed every row the window adds or replaces, under this lock.
    index: Box<dyn Index>,
    /// Bumped on every window begin: a load of an older one stops.
    epoch: u64,
    /// An overflow warning was logged; `shrink` logs only 1 (D16).
    warned: bool,
}

impl Inner {
    /// Bring a warm window up to date before a query (V2). A stale one
    /// (V3) or a failure, logged, drops it and answers `true`: warm
    /// afresh; the store answers meanwhile.
    fn refresh(&mut self, env: &Env) -> bool {
        let State::Warm(window) = &mut self.state else {
            return false;
        };
        match window.refresh(&mut self.store, &mut *self.index) {
            Ok(Fresh::Current) => return false,
            Ok(Fresh::Stale) => {}
            Err(e) => (env.log)(&e.to_string()),
        }
        self.state = State::Cold;
        true
    }

    /// The window ending at `now` fitted from catalog stats (D16).
    fn fit(&self, now: u64) -> Result<Fit, Error> {
        fit_at(&self.store, &self.cfg, now)
    }

    /// Log an overflow: `warn` and `fail` on every load, `shrink` once.
    fn report(&mut self, plan: &Plan, env: &Env) {
        let once = self.cfg.on_overflow == Overflow::Shrink;
        if !plan.over() || (once && self.warned) {
            return;
        }
        self.warned = true;
        (env.log)(&fit::overflow(plan, &self.cfg));
    }

    /// Drop the window and empty the index; a load under way stops.
    fn restart(&mut self, state: State) {
        self.epoch = self.epoch.wrapping_add(1);
        self.state = state;
    }

    /// Keep a loaded window, unless its live total, the index's bytes
    /// counted, is over the limit (V4): reported, and under `fail` the
    /// cache is off.
    fn keep(&mut self, load: Load, env: &Env) {
        let _ = (env, load.fit.live(0, &self.cfg));
        self.state = State::Warm(load.window);
    }

    /// The warm window's rows `q` wants; `None`: the store answers.
    fn view(&self, q: &Filter) -> Option<Rows<'_>> {
        let State::Warm(window) = &self.state else {
            return None;
        };
        window.view(q)
    }
}

/// A started cache; clones share it.
#[derive(Clone)]
pub struct Handle {
    inner: Arc<Mutex<Inner>>,
    env: Env,
}

/// The lock; a thread that panicked holding it is an error, not a panic.
fn lock(inner: &Mutex<Inner>) -> Result<MutexGuard<'_, Inner>, Error> {
    inner.lock().map_err(|_| Error::Poisoned)
}

impl Handle {
    fn cold(
        store: Store,
        cfg: &Config,
        env: Env,
        index: Box<dyn Index>,
    ) -> Self {
        let inner = Inner {
            store,
            cfg: cfg.cache,
            state: State::Cold,
            index,
            epoch: 0,
            warned: false,
        };
        Self {
            inner: Arc::new(Mutex::new(inner)),
            env,
        }
    }

    /// The rows `q` wants, in the store's `(ts, id)` order: from memory
    /// when the window holds all of them, else from the store (V1).
    ///
    /// # Errors
    /// A failed store read; a poisoned lock.
    pub fn query(&self, q: &Filter) -> Result<Vec<Observation>, Error> {
        self.answer(q).map(|(rows, _)| rows)
    }

    /// `f` over the rows `q` wants and the index, both lent under 1
    /// lock, nothing cloned (T6); the window is refreshed first (V2,
    /// V3). `None` whenever [`Handle::query`] would read the store:
    /// the caller reads it then.
    ///
    /// # Errors
    /// A failed refresh read; a poisoned lock.
    pub fn with_index<R>(
        &self,
        q: &Filter,
        f: impl FnOnce(Rows<'_>, &dyn Index) -> R,
    ) -> Result<Option<R>, Error> {
        let _ = (self.fresh()?, q, f);
        Ok(None)
    }

    /// Whether the window is loaded and answering.
    ///
    /// # Errors
    /// A poisoned lock.
    pub fn is_warm(&self) -> Result<bool, Error> {
        Ok(matches!(lock(&self.inner)?.state, State::Warm(_)))
    }

    /// The window's fit as the catalog stands now (`chert cache plan`).
    ///
    /// # Errors
    /// A failed catalog read; a poisoned lock.
    pub fn plan(&self) -> Result<Plan, Error> {
        Ok(lock(&self.inner)?.fit((self.env.now)())?.plan)
    }

    /// The lock, the window refreshed under it; a stale one warms
    /// afresh in the background.
    fn fresh(&self) -> Result<MutexGuard<'_, Inner>, Error> {
        let mut inner = lock(&self.inner)?;
        if inner.refresh(&self.env) {
            self.spawn(false);
        }
        Ok(inner)
    }

    fn answer(&self, q: &Filter) -> Result<(Vec<Observation>, Source), Error> {
        let mut inner = self.fresh()?;
        if let Some(rows) = inner.view(q) {
            return Ok((rows.to_vec(), Source::Cache));
        }
        Ok((inner.store.observations(q)?, Source::Store))
    }

    /// Warm on a std thread, the memory checked first on `start`; a
    /// failure is logged and leaves the store answering.
    fn spawn(&self, start: bool) {
        let handle = self.clone();
        let run = move || {
            if let Err(e) = handle.boot(start) {
                (handle.env.log)(&e.to_string());
            }
        };
        if let Err(e) = thread::Builder::new().name("cache".into()).spawn(run) {
            (self.env.log)(&format!("chert: cache not warmed: {e} (retry)"));
        }
    }

    fn boot(&self, start: bool) -> Result<(), Error> {
        if start {
            let max = lock(&self.inner)?.cfg.max_bytes;
            if let Some(text) = fit::memory(max, (self.env.memory)()) {
                (self.env.log)(&text);
            }
        }
        self.warm()
    }

    /// Begin a window and load it.
    fn warm(&self) -> Result<(), Error> {
        match self.begin()? {
            Some(load) => self.load(load),
            None => Ok(()),
        }
    }

    /// Load the window 1 partition per lock, so queries interleave; a
    /// newer begin stops it.
    fn load(&self, mut load: Load) -> Result<(), Error> {
        for path in load.window.paths() {
            if !self.step(&mut load, &path)? {
                return Ok(());
            }
        }
        let mut inner = lock(&self.inner)?;
        if inner.epoch == load.epoch {
            inner.keep(load, &self.env);
        }
        Ok(())
    }

    /// Fit the window and report it, the old one dropped and the index
    /// reset (T6); `None`: the cache is off.
    fn begin(&self) -> Result<Option<Load>, Error> {
        let mut inner = lock(&self.inner)?;
        let fit = inner.fit((self.env.now)())?;
        inner.report(&fit.plan, &self.env);
        let Some(since) = fit.since else {
            inner.restart(State::Off);
            return Ok(None);
        };
        inner.restart(State::Cold);
        let window = Window::begin(&inner.store, since)?;
        let epoch = inner.epoch;
        Ok(Some(Load { window, fit, epoch }))
    }

    /// Sync 1 partition of `load` under the lock, feeding the index;
    /// `false`: a newer begin superseded it.
    fn step(&self, load: &mut Load, path: &str) -> Result<bool, Error> {
        let mut guard = lock(&self.inner)?;
        let inner = &mut *guard;
        if inner.epoch != load.epoch {
            return Ok(false);
        }
        load.window
            .sync(&mut inner.store, &mut *inner.index, path)?;
        Ok(true)
    }
}
