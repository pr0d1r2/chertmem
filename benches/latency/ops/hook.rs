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
    let (root, stdin) = (fx.root(), payload(kind));
    let run = move || {
        let ide = Ide::ClaudeCode;
        let ctx = Ctx {
            config: &cfg,
            root: &root,
            now: END.saturating_sub(1),
            ide,
        };
        hook::run(kind, &stdin, &ctx)
    };
    measured(copy, run)
}

/// `run` once, checked, then as the measured call; `copy` lives as
/// long as it.
fn measured(
    copy: Scratch,
    mut run: impl FnMut() -> hook::Outcome + 'static,
) -> Res<Op> {
    let first = run();
    match (first.stdout, first.note) {
        (_, Some(note)) => return Err(note.into()),
        (None, None) => return Err("the hook injected nothing".into()),
        (Some(_), None) => {}
    }
    Ok(Box::new(move || {
        let _ = &copy;
        black_box(run());
        Ok(())
    }))
}

/// `session-start`: write, then inject recent sessions' gists.
pub(crate) fn session_start(fx: &Fixture) -> Res<Op> {
    injecting(fx, Kind::SessionStart, false)
}

/// `user-prompt-submit` with `inject.on_prompt = true`.
pub(crate) fn prompt(fx: &Fixture) -> Res<Op> {
    injecting(fx, Kind::UserPromptSubmit, true)
}
