// Only `#[test]` fns here, imports inside them: `cargo bench` builds
// this module with `--cfg test` and strips every test (lib.rs).

#[test]
fn the_newest_partition_is_the_last_week() -> crate::fixture::Res<()> {
    use crate::fixture::{END, Scratch, Spec, WEEK, cached_in};
    let base = Scratch::new("read-newest")?;
    let fx = cached_in(base.path(), Spec::new(21, 400, 4))?;
    let mut store = fx.store()?;
    let filter = super::newest(&store);
    assert_eq!(filter.since, Some(END - WEEK));
    let rows = store.observations(&filter)?.len();
    assert!((90..=110).contains(&rows), "{rows}");
    assert_eq!(super::sample(&mut store)?.len(), super::REFS);
    Ok(())
}

#[test]
fn get_scan_and_warm_up_run_on_a_fixture() -> crate::fixture::Res<()> {
    use crate::fixture::{Scratch, Spec, cached_in};
    let base = Scratch::new("read-ops")?;
    let fx = cached_in(base.path(), Spec::new(22, 200, 2))?;
    for make in [super::get, super::scan, super::warm_up] {
        let mut op = make(&fx)?;
        op()?;
        op()?;
    }
    Ok(())
}

#[test]
fn an_empty_newest_partition_is_refused() -> crate::fixture::Res<()> {
    use crate::fixture::{Scratch, Spec, cached_in};
    let base = Scratch::new("read-empty")?;
    let fx = cached_in(base.path(), Spec::new(23, 0, 1))?;
    assert!(super::scan(&fx).is_err());
    assert!(super::get(&fx).is_ok_and(|mut op| op().is_ok()));
    Ok(())
}
