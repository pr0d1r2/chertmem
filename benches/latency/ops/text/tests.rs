// Only `#[test]` fns here, imports inside them: `cargo bench` builds
// this module with `--cfg test` and strips every test (lib.rs).

#[test]
fn every_event_body_is_compressed_then_expanded() -> crate::fixture::Res<()> {
    use crate::fixture::{Fixture, Spec};
    // Text suites read the spec only: no store is opened.
    let dir = std::env::temp_dir().join("chert-bench-text-unused");
    let fx = Fixture {
        dir,
        spec: Spec::new(50, 150, 1),
    };
    assert_eq!(super::bodies(&fx)?.len(), 150);
    for make in [super::compress, super::expand] {
        let mut op = make(&fx)?;
        op()?;
    }
    assert!(!fx.dir.exists());
    Ok(())
}

#[test]
fn a_spec_without_events_is_refused() {
    use crate::fixture::{Fixture, Spec};
    let dir = std::env::temp_dir().join("chert-bench-text-unused");
    let fx = Fixture {
        dir,
        spec: Spec::new(51, 0, 1),
    };
    assert!(super::compress(&fx).is_err());
    assert!(super::expand(&fx).is_err());
}
