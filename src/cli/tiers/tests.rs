//! `chert partitions` and `chert cache plan`: exit 0, `cache plan` 1
//! when the window's estimate is over `cache.max_bytes` (V2).

use super::super::tests::{Res, SESSION, Sandbox, Test, first_ref};

/// The path of the sandbox's only partition.
fn only(s: &Sandbox) -> Res<String> {
    let ran = s.run(&["partitions", "list"]);
    assert_eq!((ran.code, ran.out.lines().count()), (0, 1), "{ran:?}");
    let path = ran.out.split_whitespace().next().ok_or("no partition")?;
    Ok(path.to_owned())
}

#[test]
fn partitions_lists_each_partition_by_default() -> Test {
    let s = Sandbox::new("partitions-list")?;
    assert_eq!(s.run(&["partitions"]), s.run(&["partitions", "list"]));
    assert_eq!(s.run(&["partitions"]).out, "");
    s.hook("post-tool-use")?;
    let ran = s.run(&["partitions"]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert!(ran.out.starts_with("chunks/"), "{ran:?}");
    assert!(ran.out.contains(" hot 1 rows "), "{ran:?}");
    Ok(())
}

#[test]
fn seal_then_compact_move_a_partition_down_the_tiers() -> Test {
    let s = Sandbox::new("partitions-tiers")?;
    s.hook("post-tool-use")?;
    let path = only(&s)?;
    assert_eq!(s.run(&["partitions", "seal", &path]).code, 0);
    assert!(s.run(&["partitions"]).out.contains(" warm "));
    assert_eq!(s.run(&["partitions", "compact", &path]).code, 0);
    assert!(s.run(&["partitions"]).out.contains(" cold "));
    Ok(())
}

#[test]
fn drop_leaves_its_refs_dropped() -> Test {
    let s = Sandbox::new("partitions-drop")?;
    s.hook("post-tool-use")?;
    let reference = first_ref(&s.run(&["timeline", SESSION]).out)?;
    let path = only(&s)?;
    assert_eq!(s.run(&["partitions", "drop", &path]).code, 0);
    assert_eq!(s.run(&["get", &reference]).code, 1);
    Ok(())
}

#[test]
fn repartition_under_a_new_spec_is_exit_0() -> Test {
    let s = Sandbox::new("partitions-repartition")?;
    s.hook("post-tool-use")?;
    s.project_toml("[partition]\ntime_bucket = \"14d\"\n")?;
    let ran = s.run(&["partitions", "repartition"]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert!(only(&s)?.ends_with("-14d.db"), "{:?}", only(&s));
    Ok(())
}

#[test]
fn cache_plan_under_the_limit_is_exit_0() -> Test {
    let s = Sandbox::new("cache-plan")?;
    s.hook("post-tool-use")?;
    assert_eq!(s.run(&["cache"]).out, s.run(&["cache", "plan"]).out);
    let ran = s.run(&["cache", "plan"]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert!(ran.out.contains("limit 268435456 bytes"), "{ran:?}");
    Ok(())
}

#[test]
fn cache_plan_over_the_limit_is_exit_1() -> Test {
    let s = Sandbox::new("cache-plan-over")?;
    s.project_toml("[cache]\nmax_bytes = \"1KiB\"\n")?;
    s.hook("post-tool-use")?;
    s.hook("stop")?;
    let ran = s.run(&["cache", "plan"]);
    assert_eq!(ran.code, 1, "{ran:?}");
    assert!(ran.out.contains("limit 1024 bytes"), "{ran:?}");
    Ok(())
}
