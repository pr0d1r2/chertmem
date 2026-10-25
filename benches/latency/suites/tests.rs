// Only `#[test]` fns here, imports inside them: `cargo bench` builds
// this module with `--cfg test` and strips every test (lib.rs).
//
// benches V1: the targets and suites are read from the specs
// themselves, so a target or suite added there without a suite here
// fails.

#[test]
fn every_root_latency_target_has_a_suite_at_its_budget()
-> crate::fixture::Res<()> {
    let spec = include_str!("../../../SPEC.md");
    let line = spec.lines().find(|l| l.starts_with("- latency targets"));
    let (_, list) = line.unwrap_or_default().split_once(": ").ok_or("none")?;
    let items: Vec<&str> = list.trim_end_matches('.').split(" · ").collect();
    assert_eq!(items.len(), 4, "{list}");
    for item in items {
        let (name, budget) = item.split_once(" < ").ok_or("no budget")?;
        let name = name.replace('`', "");
        let ms: u64 = budget.trim_end_matches(" ms").parse()?;
        let measured = super::ALL.iter().filter_map(|s| s.target);
        let found: Vec<_> = measured.filter(|t| t.name == name).collect();
        assert!(!found.is_empty(), "root C target {name:?} has no suite");
        assert!(found.iter().all(|t| t.ms == ms), "{name}: {found:?}");
    }
    Ok(())
}

#[test]
fn every_suite_benches_spec_names_is_registered_and_no_other()
-> crate::fixture::Res<()> {
    use super::ALL;
    let spec = include_str!("../../SPEC.md");
    let line = spec.lines().find_map(|l| l.strip_prefix("- suites: "));
    let named: Vec<String> = line
        .ok_or("no suites")?
        .trim_end_matches('.')
        .split(" · ")
        .map(|s| s.split(" (").next().unwrap_or(s).replace('`', ""))
        .collect();
    assert_eq!(named.len(), 10, "{named:?}");
    for name in &named {
        assert!(ALL.iter().any(|s| s.spec == name), "{name:?} has no suite");
    }
    assert!(ALL.iter().all(|s| named.iter().any(|n| n == s.spec)));
    Ok(())
}

#[test]
fn only_the_hook_write_suite_waits_and_it_says_for_what() {
    use super::{ALL, Status};
    let pending: Vec<(&str, &str)> = ALL
        .iter()
        .filter_map(|s| match s.status {
            Status::Pending(why) => Some((s.id, why)),
            Status::Ready(_) => None,
        })
        .collect();
    assert_eq!(pending.len(), 1);
    let (id, why) = pending.first().copied().unwrap_or_default();
    assert_eq!(id, "hook_write");
    assert!(why.contains("src/cli") && why.contains("V4"), "{why}");
}

#[test]
fn suite_ids_are_distinct_criterion_groups() {
    let ids: std::collections::BTreeSet<&str> =
        super::ALL.iter().map(|s| s.id).collect();
    assert_eq!(ids.len(), super::ALL.len());
    let plain =
        |id: &&str| id.chars().all(|c| c.is_ascii_lowercase() || c == '_');
    assert!(ids.iter().all(plain));
}

#[test]
fn the_store_grid_holds_every_stated_size_and_partition_count() {
    // benches C: 1k, 10k, 100k, 1M observations; 1, 4, 52, 520 parts.
    use super::{ALL, Grid, grid};
    let store = ALL.iter().find(|s| s.grid == Grid::Store);
    let specs = store.map(grid).unwrap_or_default();
    for size in [1000, 10_000, 100_000, 1_000_000] {
        assert!(specs.iter().any(|s| s.size == size), "{size}");
    }
    for parts in [1, 4, 52, 520] {
        assert!(specs.iter().any(|s| s.parts == parts), "{parts}");
    }
    let sound =
        |s: &crate::fixture::Spec| s.size >= s.parts && s.seed == super::SEED;
    assert!(specs.iter().all(sound));
    let text = ALL.iter().find(|s| s.grid == Grid::Text);
    assert_eq!(text.map(grid).map(|g| g.len()), Some(1));
}
