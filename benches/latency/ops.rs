//! The measured calls behind every ready suite, through the library's
//! own entry points (benches T3), and what they share: the fixture's
//! clock and a warm cache.

#[cfg(test)]
mod tests;

pub(crate) mod hook;
pub(crate) mod read;
pub(crate) mod search;
pub(crate) mod text;

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use chertmem::cache::{Cache, Env, Handle};
use chertmem::config::Config;
use chertmem::store::{Mode, Store};

use crate::fixture::{END, Fixture, Res};

/// The longest a cache may take to warm before the bench gives up.
const WARM_LIMIT: Duration = Duration::from_mins(5);

/// The cache's host: the clock at [`END`], no memory check, no log.
pub(crate) fn env() -> Env {
    Env {
        now: || END,
        memory: || None,
        log: Arc::new(|_: &str| {}),
    }
}

/// A cache of `fx` started now.
pub(crate) fn start(fx: &Fixture, cfg: &Config) -> Res<Handle> {
    let store = Store::open(cfg, fx.root(), Mode::Read)?;
    Ok(Cache::start_with(store, cfg, env()))
}

/// Wait for `cache` to be warm.
pub(crate) fn wait(cache: &Handle) -> Res<()> {
    let begun = Instant::now();
    while !cache.is_warm()? {
        if begun.elapsed() > WARM_LIMIT {
            return Err("the cache never warmed".into());
        }
        thread::sleep(Duration::from_micros(50));
    }
    Ok(())
}

/// A warm cache of `fx`.
pub(crate) fn warm(fx: &Fixture, cfg: &Config) -> Res<Handle> {
    let cache = start(fx, cfg)?;
    wait(&cache)?;
    Ok(cache)
}
