//! The hook-write suite: a real `chert hook run post-tool-use` process
//! per call (benches V4), so its startup is counted.

#![allow(
    dead_code,
    clippy::unused_self,
    reason = "RED stub: `cargo bench` strips its tests; built next commit"
)]

#[cfg(test)]
mod tests;

#[cfg(test)]
use crate::fixture::{Fixture, Res};

/// RED: not built yet.
#[cfg(test)]
pub(crate) const PAYLOAD: &[u8] = b"";

/// RED: not built yet.
#[cfg(test)]
pub(crate) struct Process;

#[cfg(test)]
impl Process {
    pub(crate) fn of(_fx: &Fixture) -> Res<Self> {
        Err("not built".into())
    }

    pub(crate) fn run(&self) -> Res<()> {
        Err("not built".into())
    }

    pub(crate) fn run_with(&self, _stdin: &[u8]) -> Res<()> {
        Err("not built".into())
    }

    pub(crate) fn rows(&self) -> Res<usize> {
        Err("not built".into())
    }
}
