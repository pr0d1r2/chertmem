// Only `#[test]` fns here, imports inside them: `cargo bench` builds
// this module with `--cfg test` and strips every test (lib.rs).

#[test]
fn the_clock_stands_at_the_fixtures_end() {
    let env = super::env();
    assert_eq!((env.now)(), crate::fixture::END);
    assert_eq!((env.memory)(), None);
    (env.log)("dropped");
}

#[test]
fn a_cache_of_a_fixture_warms() -> crate::fixture::Res<()> {
    use crate::fixture::{Scratch, Spec, cached_in};
    let base = Scratch::new("ops-warm")?;
    let fx = cached_in(base.path(), Spec::new(20, 200, 4))?;
    let cache = super::warm(&fx, &fx.config()?)?;
    assert!(cache.is_warm()?);
    Ok(())
}
