// Only `#[test]` fns here, imports inside them: `cargo bench` builds
// this module with `--cfg test` and strips every test (lib.rs).

#[test]
fn the_query_is_2_common_words() {
    assert_eq!(super::text(), "cache partition");
}

#[test]
fn keyword_searches_run_on_a_fixture() -> crate::fixture::Res<()> {
    use crate::fixture::{Scratch, Spec, cached_in};
    let base = Scratch::new("search-ops")?;
    let fx = cached_in(base.path(), Spec::new(30, 600, 8))?;
    for make in [super::recent_warm, super::recent_cold, super::cross] {
        let mut op = make(&fx)?;
        op()?;
    }
    Ok(())
}

#[test]
fn a_query_that_finds_nothing_is_refused() -> crate::fixture::Res<()> {
    use crate::fixture::{Scratch, Spec, cached_in};
    let base = Scratch::new("search-none")?;
    let fx = cached_in(base.path(), Spec::new(31, 0, 1))?;
    let failed = super::cross(&fx).err().map(|e| e.to_string());
    let said = r#""cache partition" found nothing"#;
    assert_eq!(failed.as_deref(), Some(said));
    Ok(())
}

#[test]
fn the_semantic_suite_runs_against_the_stand_in_ollama()
-> crate::fixture::Res<()> {
    // `Engine` reads OLLAMA_HOST only from the environment, so the
    // check runs in a child test with it set, as `main.rs` does.
    let mark = crate::fixture::Scratch::new("search-semantic-mark")?;
    let file = mark.path().join("ran");
    let mut child = std::process::Command::new(std::env::current_exe()?);
    child.args(["--ignored", "--exact", "ops::search::tests::semantic_child"]);
    child.env("CHERT_BENCH_MARK", &file);
    assert_eq!(crate::ollama::serve_for(child)?, 0);
    assert_eq!(std::fs::read_to_string(&file)?, "ran");
    Ok(())
}

#[test]
#[ignore = "run by the_semantic_suite_runs_against_the_stand_in_ollama"]
fn semantic_child() -> crate::fixture::Res<()> {
    use crate::fixture::{Scratch, Spec, cached_in};
    let mark = std::env::var_os("CHERT_BENCH_MARK").ok_or("no mark")?;
    let base = Scratch::new("search-semantic")?;
    let fx = cached_in(base.path(), Spec::new(32, 300, 4))?;
    let mut op = super::semantic(&fx)?;
    op()?;
    Ok(std::fs::write(mark, "ran")?)
}
