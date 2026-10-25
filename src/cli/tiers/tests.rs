//! `chert partitions` and `chert cache plan`: exit 0, `cache plan` 1
//! when the window's estimate is over `cache.max_bytes` (V2).

use super::super::tests::{
    CHERT, NOW, Ran, Res, SESSION, Sandbox, Test, first_ref,
};
use super::super::{Io, run};

/// 8 days past [`NOW`]: the week of every hook a sandbox runs is over.
const LATER: u64 = NOW + 8 * 86_400_000;

fn later() -> u64 {
    LATER
}

/// `chert <args>` in the sandbox, as [`Sandbox::run`], at [`LATER`].
fn run_later(s: &Sandbox, args: &[&str]) -> Ran {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = exec(s, args, &mut out, &mut err);
    let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    Ran {
        code,
        out: text(&out),
        err: text(&err),
    }
}

/// Run `chert <args>` at [`LATER`] with empty stdin: its exit code.
fn exec(
    s: &Sandbox,
    args: &[&str],
    out: &mut Vec<u8>,
    err: &mut Vec<u8>,
) -> u8 {
    let (home, project, mut input) = (s.home(), s.project(), &b""[..]);
    let mut io = Io {
        stdin: &mut input,
        stdout: out,
        stderr: err,
        cwd: &project,
        home: Some(&home),
        now: later,
        chert: CHERT,
        tty: super::super::Tty::default(),
    };
    run(std::iter::once("chert").chain(args.to_vec()), &mut io)
}

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
    assert_eq!(run_later(&s, &["partitions", "seal", &path]).code, 0);
    assert!(s.run(&["partitions"]).out.contains(" warm "));
    assert_eq!(run_later(&s, &["partitions", "compact", &path]).code, 0);
    assert!(s.run(&["partitions"]).out.contains(" cold "));
    Ok(())
}

#[test]
fn sealing_or_compacting_the_open_week_is_exit_2_naming_its_end() -> Test {
    let s = Sandbox::new("partitions-open")?;
    s.hook("post-tool-use")?;
    let path = only(&s)?;
    for verb in ["seal", "compact"] {
        let ran = s.run(&["partitions", verb, &path]);
        assert_eq!((ran.code, ran.out.as_str()), (2, ""), "{ran:?}");
        let end = "open until 2026-10-29T00:00:00Z (1793232000000 ms)";
        assert!(ran.err.contains(end), "{ran:?}");
    }
    assert!(s.run(&["partitions"]).out.contains(" hot 1 rows "));
    Ok(())
}

#[test]
fn an_event_past_a_compacted_then_dropped_week_is_stored() -> Test {
    let s = Sandbox::new("partitions-b3")?;
    s.hook("post-tool-use")?;
    let path = only(&s)?;
    for verb in ["compact", "drop"] {
        assert_eq!(run_later(&s, &["partitions", verb, &path]).code, 0);
    }
    s.hook("session-start")?;
    let fresh = path.replace(".db", "~1.db");
    assert_eq!(only(&s)?, fresh);
    assert!(s.run(&["partitions"]).out.contains(" hot 1 rows "));
    let reference = first_ref(&s.run(&["timeline", SESSION]).out)?;
    assert_eq!(s.run(&["get", &reference]).code, 0);
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
