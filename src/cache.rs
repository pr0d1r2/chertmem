//! The recent `cache.window` of the current project in memory inside
//! `chert mcp` (src/cache G, D14): warmed on a background thread, the
//! store answering until then. The store stays the truth (root C).

#[cfg(test)]
mod tests;

mod error;
mod fit;
mod window;

use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

pub use error::Error;

use crate::config::{self, Config};
use crate::store::{Filter, Observation, Store};
use window::{Fresh, Window};

/// Where warnings go: the MCP log, `status`, `doctor`; never into a
/// query result (src/cache V4, root V11).
pub type Log = Arc<dyn Fn(&str) + Send + Sync>;

/// What a cache takes from its host; injected so tests own the clock.
#[derive(Clone)]
pub struct Env {
    /// Milliseconds since the Unix epoch.
    pub now: fn() -> u64,
    pub log: Log,
}

impl Env {
    /// The system clock; warnings on stderr, which `chert mcp` keeps
    /// apart from its stdio protocol.
    #[must_use]
    pub fn system() -> Self {
        Self {
            now: clock,
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
    /// Warm `store`'s window on a background thread; until it is warm,
    /// queries go to the store (src/cache C).
    #[must_use]
    pub fn start(store: Store, cfg: &Config) -> Handle {
        Self::start_with(store, cfg, Env::system())
    }

    /// [`Cache::start`] with the clock and log sink given.
    #[must_use]
    pub fn start_with(store: Store, cfg: &Config, env: Env) -> Handle {
        let handle = Handle::cold(store, cfg, env);
        handle.spawn();
        handle
    }
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
}

struct Inner {
    store: Store,
    cfg: config::Cache,
    state: State,
}

impl Inner {
    /// Bring a warm window up to date before a query (V2). A stale one
    /// (V3) or a failure, logged, drops it and answers `true`: warm
    /// afresh; the store answers meanwhile.
    fn refresh(&mut self, env: &Env) -> bool {
        let State::Warm(window) = &mut self.state else {
            return false;
        };
        match window.refresh(&mut self.store) {
            // RED stub: a stale window keeps answering.
            Ok(Fresh::Current | Fresh::Stale) => return false,
            Err(e) => (env.log)(&e.to_string()),
        }
        self.state = State::Cold;
        true
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
    fn cold(store: Store, cfg: &Config, env: Env) -> Self {
        let inner = Inner {
            store,
            cfg: cfg.cache,
            state: State::Cold,
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

    /// Whether the window is loaded and answering.
    ///
    /// # Errors
    /// A poisoned lock.
    pub fn is_warm(&self) -> Result<bool, Error> {
        Ok(matches!(lock(&self.inner)?.state, State::Warm(_)))
    }

    fn answer(&self, q: &Filter) -> Result<(Vec<Observation>, Source), Error> {
        let mut inner = lock(&self.inner)?;
        if inner.refresh(&self.env) {
            self.spawn();
        }
        if let State::Warm(window) = &inner.state
            && let Some(rows) = window.answer(q)
        {
            return Ok((rows, Source::Cache));
        }
        Ok((inner.store.observations(q)?, Source::Store))
    }

    /// Warm on a std thread; a failure is logged and leaves the store
    /// answering.
    fn spawn(&self) {
        let handle = self.clone();
        let run = move || {
            if let Err(e) = handle.warm() {
                (handle.env.log)(&e.to_string());
            }
        };
        if let Err(e) = thread::Builder::new().name("cache".into()).spawn(run) {
            (self.env.log)(&format!("chert: cache not warmed: {e} (retry)"));
        }
    }

    /// Load the window 1 partition per lock, so queries interleave.
    fn warm(&self) -> Result<(), Error> {
        let mut window = self.begin()?;
        for path in window.paths() {
            window.sync(&mut lock(&self.inner)?.store, &path)?;
        }
        lock(&self.inner)?.state = State::Warm(window);
        Ok(())
    }

    fn begin(&self) -> Result<Window, Error> {
        let inner = lock(&self.inner)?;
        let span = fit::window_ms(inner.cfg.window);
        let since = (self.env.now)().saturating_sub(span);
        Ok(Window::begin(&inner.store, since)?)
    }
}
