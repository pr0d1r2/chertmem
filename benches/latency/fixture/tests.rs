// Only `#[test]` fns here, imports inside them: `cargo bench` builds
// this module with `--cfg test` and strips every test (lib.rs).

#[test]
fn labels_name_the_size_by_its_largest_whole_unit() {
    use super::Spec;
    assert_eq!(Spec::new(1, 1000, 52).label(), "1k-p52");
    assert_eq!(Spec::new(1, 1_000_000, 4).label(), "1M-p4");
    assert_eq!(Spec::new(1, 100_000, 520).label(), "100k-p520");
    assert_eq!(Spec::new(1, 250, 1).label(), "250-p1");
    assert_eq!(Spec::new(1, 1500, 1).label(), "1500-p1");
    assert_eq!(Spec::new(1, 10_000, 52).key(), "1-10000-52");
}

#[test]
fn events_spread_over_every_week_before_the_end() {
    use super::{END, Spec, WEEK};
    let spec = Spec::new(1, 1000, 520);
    let weeks: std::collections::BTreeSet<u64> =
        (0..spec.size).map(|i| spec.ts(i) / WEEK).collect();
    assert_eq!(weeks.len(), 520);
    assert_eq!(spec.ts(0), END - 520 * WEEK);
    assert!(spec.ts(999) < END);
    assert_eq!(END % WEEK, 0);
}

#[test]
fn the_same_seed_gives_the_same_digest_another_seed_another() {
    // benches V2: the rows are the fixture's bytes.
    use super::{Rows, Spec, digest};
    let of = |seed, size| digest(Rows::new(Spec::new(seed, size, 4)));
    assert_eq!(of(9, 500), of(9, 500));
    assert_ne!(of(9, 500), of(10, 500));
    assert_ne!(of(9, 500), of(9, 501));
}

#[test]
fn a_built_fixture_holds_every_row_in_its_partitions() -> super::Res<()> {
    use super::{Scratch, Spec, cached_in};
    use chertmem::store::Filter;
    let base = Scratch::new("fixture-built")?;
    let fx = cached_in(base.path(), Spec::new(3, 300, 4))?;
    let mut store = fx.store()?;
    assert_eq!(store.partitions(&Filter::default())?.len(), 4);
    assert_eq!(store.observations(&Filter::default())?.len(), 300);
    assert!(!store.sessions(1000)?.is_empty());
    Ok(())
}

#[test]
fn a_fixture_is_stamped_with_generator_key_and_digest() -> super::Res<()> {
    use super::{GENERATOR, Rows, Scratch, Spec, cached_in, digest};
    let base = Scratch::new("fixture-stamp")?;
    let spec = Spec::new(4, 100, 2);
    let fx = cached_in(base.path(), spec)?;
    let stamp = std::fs::read_to_string(fx.dir.join("stamp"))?;
    let hash = digest(Rows::new(spec));
    assert_eq!(stamp, format!("{GENERATOR} 4-100-2 {hash:016x}\n"));
    Ok(())
}

#[test]
fn a_cached_fixture_is_kept_and_another_generators_rebuilt() -> super::Res<()> {
    use super::{Scratch, Spec, cached_in};
    let base = Scratch::new("fixture-cache")?;
    let spec = Spec::new(4, 100, 2);
    let marker = cached_in(base.path(), spec)?.dir.join("marker");
    std::fs::write(&marker, "kept")?;
    let fx = cached_in(base.path(), spec)?;
    assert!(marker.exists(), "a fresh stamp keeps the fixture");
    std::fs::write(fx.dir.join("stamp"), "fixture-0 4-100-2 0\n")?;
    cached_in(base.path(), spec)?;
    assert!(!marker.exists(), "an old generator's fixture is rebuilt");
    Ok(())
}

#[test]
fn a_scratch_copy_is_writable_and_gone_on_drop() -> super::Res<()> {
    use super::{Scratch, Spec, cached_in};
    let base = Scratch::new("fixture-scratch")?;
    let fx = cached_in(base.path(), Spec::new(5, 50, 1))?;
    let (copy, cfg) = Scratch::of(&fx, "fixture-scratch-copy")?;
    let dir = copy.path().to_path_buf();
    assert!(dir.join("catalog.db").exists());
    assert_eq!(cfg.storage.data_dir.as_deref(), Some(dir.as_path()));
    drop(copy);
    assert!(!dir.exists());
    Ok(())
}

#[test]
fn fixtures_are_cached_under_the_target_dir() {
    let base = super::base();
    assert!(base.ends_with("bench-fixtures"), "{}", base.display());
}

#[test]
fn a_copied_fixture_finds_its_rows_unbuilt() -> super::Res<()> {
    // benches T5: the project id must not hash the path it was built at.
    use super::{Scratch, Spec, cached_in, copy_tree};
    use crate::ops::read::newest;
    let (from, to) = (Scratch::new("fx-from")?, Scratch::new("fx-to")?);
    let spec = Spec::new(6, 200, 2);
    let built = cached_in(from.path(), spec)?;
    std::fs::write(built.dir.join("marker"), "kept")?;
    copy_tree(&built.dir, &to.path().join(spec.key()))?;
    drop(from);
    let fx = cached_in(to.path(), spec)?;
    assert!(fx.dir.join("marker").exists(), "a copy is not rebuilt");
    let mut store = fx.store()?;
    let rows = store.observations(&newest(&store))?;
    assert!(!rows.is_empty(), "the copy has rows");
    Ok(())
}
