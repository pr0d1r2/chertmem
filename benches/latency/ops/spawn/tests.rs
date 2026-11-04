// Only `#[test]` fns here, imports inside them: `cargo bench` builds
// this module with `--cfg test` and strips every test (lib.rs).

#[test]
fn a_real_chert_process_stores_1_row_per_run() -> crate::fixture::Res<()> {
    // benches V4: the suite spawns `chert hook run`, startup counted.
    use crate::fixture::{Scratch, Spec, cached_in};
    let base = Scratch::new("spawn-rows")?;
    let fx = cached_in(base.path(), Spec::new(42, 400, 4))?;
    let process = super::Process::of(&fx)?;
    assert_eq!(process.rows()?, 0, "the process's project starts empty");
    process.run()?;
    process.run()?;
    assert_eq!(process.rows()?, 2);
    let all = chertmem::store::Filter::default();
    let rows = fx.store()?.observations(&all)?.len();
    assert_eq!(rows, 400, "the cached fixture stays as built");
    Ok(())
}

#[test]
fn a_process_that_stores_nothing_fails_the_run() -> crate::fixture::Res<()> {
    // `chert hook run` notes a bad payload on stderr and exits 0 (src/cli
    // R1); timing it would time a write that never happened.
    use crate::fixture::{Scratch, Spec, cached_in};
    let base = Scratch::new("spawn-refused")?;
    let fx = cached_in(base.path(), Spec::new(43, 40, 1))?;
    let process = super::Process::of(&fx)?;
    let err = process.run_with(b"not json").err().map(|e| e.to_string());
    assert!(
        err.is_some_and(|e| e.contains("not stored")),
        "a note fails it"
    );
    assert_eq!(process.rows()?, 0);
    Ok(())
}

#[test]
fn the_payload_is_a_claude_code_post_tool_use() -> crate::fixture::Res<()> {
    use serde_json::Value;
    let payload: Value = serde_json::from_slice(super::PAYLOAD)?;
    assert_eq!(
        payload.get("tool_name").and_then(Value::as_str),
        Some("Bash")
    );
    assert!(payload.get("session_id").is_some());
    Ok(())
}
