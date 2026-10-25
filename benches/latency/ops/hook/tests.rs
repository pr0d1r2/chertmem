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
