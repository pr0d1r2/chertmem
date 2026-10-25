// Only `#[test]` fns here, imports inside them: `cargo bench` builds
// this module with `--cfg test` and strips every test (lib.rs).

#[test]
fn payloads_are_claude_code_json() -> crate::fixture::Res<()> {
    use chertmem::hook::Kind;
    use serde_json::Value;
    let start: Value =
        serde_json::from_slice(&super::payload(Kind::SessionStart))?;
    let prompt: Value =
        serde_json::from_slice(&super::payload(Kind::UserPromptSubmit))?;
    assert_eq!(start.get("source").and_then(Value::as_str), Some("startup"));
    assert!(prompt.get("prompt").is_some());
    assert_eq!(prompt.get("session_id"), start.get("session_id"));
    Ok(())
}

#[test]
fn both_injecting_hooks_run_on_a_copy() -> crate::fixture::Res<()> {
    use crate::fixture::{Scratch, Spec, cached_in};
    let base = Scratch::new("hook-ops")?;
    let fx = cached_in(base.path(), Spec::new(40, 400, 4))?;
    for make in [super::session_start, super::prompt] {
        let mut op = make(&fx)?;
        op()?;
    }
    let all = chertmem::store::Filter::default();
    let rows = fx.store()?.observations(&all)?.len();
    assert_eq!(rows, 400, "the cached fixture stays as built");
    Ok(())
}

#[test]
fn a_hook_that_injects_nothing_or_notes_is_refused() -> crate::fixture::Res<()>
{
    let outcome =
        |stdout: Option<&str>, note: Option<&str>| chertmem::hook::Outcome {
            stdout: stdout.map(str::to_owned),
            exit: 0,
            note: note.map(str::to_owned),
        };
    let copy = || crate::fixture::Scratch::new("hook-refused");
    assert!(super::measured(copy()?, move || outcome(None, None)).is_err());
    let noted =
        super::measured(copy()?, move || outcome(Some("x"), Some("bad")));
    assert_eq!(noted.err().map(|e| e.to_string()).as_deref(), Some("bad"));
    let mut fine = super::measured(copy()?, move || outcome(Some("x"), None))?;
    fine()
}

#[test]
fn a_note_on_any_measured_run_fails_it() -> crate::fixture::Res<()> {
    // benches T5: a hook that stops writing mid-run must not be timed as
    // a fast one.
    let mut runs = 0_u32;
    let run = move || {
        runs = runs.saturating_add(1);
        chertmem::hook::Outcome {
            stdout: Some("x".to_owned()),
            exit: 0,
            note: (runs > 1).then(|| "no new id".to_owned()),
        }
    };
    let copy = crate::fixture::Scratch::new("hook-noted-later")?;
    let mut op = super::measured(copy, run)?;
    let got = op().err().map(|e| e.to_string());
    assert_eq!(got.as_deref(), Some("no new id"));
    Ok(())
}

#[test]
fn an_injecting_hook_outlasts_one_ms_worth_of_ids() -> crate::fixture::Res<()> {
    // benches T5: a ms holds at least 768 ids (src/store R6); 2000 runs
    // at one fixed ms ran out.
    use crate::fixture::{Scratch, Spec, cached_in};
    let base = Scratch::new("hook-ids")?;
    let fx = cached_in(base.path(), Spec::new(41, 40, 1))?;
    let mut op = super::session_start(&fx)?;
    for _ in 0..2000 {
        op()?;
    }
    Ok(())
}

#[test]
fn the_clock_moves_back_1_ms_a_run_inside_the_newest_week() {
    use crate::fixture::{END, WEEK};
    let mut clock = super::Clock::default();
    let ticks: Vec<u64> = (0..3).map(|_| clock.tick()).collect();
    assert_eq!(ticks, [END - 1, END - 2, END - 3]);
    assert_eq!((END - 1) / WEEK, (END - 1_000_000) / WEEK);
}
