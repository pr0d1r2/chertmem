//! The injecting hooks in process, through `hook::run` (src/hook):
//! `session-start` by default, `user-prompt-submit` with the opt-in
//! `inject.on_prompt`. Each run appends 1 row, so each suite works on a
//! copy of the fixture. The process start these leave out is the
//! pending hook-write suite's.

#[cfg(test)]
mod tests;

use std::hint::black_box;

use chertmem::hook::{self, Ctx, Ide, Kind};

use crate::fixture::{END, Fixture, Res, Scratch};
use crate::suites::Op;

/// The payload of `kind` in Claude Code's dialect.
pub(crate) fn payload(kind: Kind) -> Vec<u8> {
    let body = match kind {
        Kind::UserPromptSubmit => {
            r#""prompt":"why does the cache partition test fail""#
        }
        _ => r#""source":"startup""#,
    };
    format!(r#"{{"session_id":"bench","cwd":"/work/project",{body}}}"#)
        .into_bytes()
}

/// Hook `kind` on a copy of `fx`, `on_prompt` as given; it must inject
/// once before it is measured.
fn injecting(fx: &Fixture, kind: Kind, on_prompt: bool) -> Res<Op> {
    let name = format!("{}-{}", kind.as_str(), fx.spec.key());
    let (copy, mut cfg) = Scratch::of(fx, &name)?;
    cfg.inject.on_prompt = on_prompt;
    let (root, stdin, mut clock) = (fx.root(), payload(kind), Clock::default());
    let run = move || {
        let ctx = Ctx {
            config: &cfg,
            root,
            now: clock.tick(),
            ide: Ide::ClaudeCode,
        };
        hook::run(kind, &stdin, &ctx)
    };
    measured(copy, run)
}

/// The injecting hooks' clock (benches T5): 1 ms earlier each run,
/// from just before [`END`], so no ms is asked for more ids than the
/// store holds in one (src/store R6). 10^6 runs move it 17 minutes,
/// still inside the newest weekly partition.
#[derive(Debug, Default)]
pub(crate) struct Clock(u64);

impl Clock {
    /// This run's time; the next is 1 ms earlier.
    pub(crate) fn tick(&mut self) -> u64 {
        let now = END.saturating_sub(1).saturating_sub(self.0);
        self.0 = self.0.saturating_add(1);
        now
    }
}

/// `run` once, checked, then as the measured call, every run checked
/// for a note; `copy` lives as long as it.
fn measured(
    copy: Scratch,
    mut run: impl FnMut() -> hook::Outcome + 'static,
) -> Res<Op> {
    match checked(run())? {
        Some(_) => {}
        None => return Err("the hook injected nothing".into()),
    }
    Ok(Box::new(move || {
        let _ = &copy;
        black_box(checked(run())?);
        Ok(())
    }))
}

/// The hook's context, or its note as the error: a hook that noted did
/// not write as measured.
fn checked(out: hook::Outcome) -> Res<Option<String>> {
    match out.note {
        Some(note) => Err(note.into()),
        None => Ok(out.stdout),
    }
}

/// `session-start`: write, then inject recent sessions' gists.
pub(crate) fn session_start(fx: &Fixture) -> Res<Op> {
    injecting(fx, Kind::SessionStart, false)
}

/// `user-prompt-submit` with `inject.on_prompt = true`.
pub(crate) fn prompt(fx: &Fixture) -> Res<Op> {
    injecting(fx, Kind::UserPromptSubmit, true)
}
