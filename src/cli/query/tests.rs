//! `chert search`, `timeline` and `get`: exit 0 with rows or none, 1
//! for a dropped ref, 2 for a ref that is not `chert:<ref>` (V2).

use super::super::tests::{NOW, Res, SESSION, Sandbox, Test, first_ref};

/// A sandbox holding the recorded `post-tool-use` (`echo chert-fixture`).
fn stored(name: &str) -> Res<Sandbox> {
    let s = Sandbox::new(name)?;
    s.hook("post-tool-use")?;
    Ok(s)
}

#[test]
fn search_prints_1_line_per_hit_with_exit_0() -> Test {
    let s = stored("search-hit")?;
    let ran = s.run(&["search", "chert-fixture", "--limit", "5"]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert_eq!(ran.out.lines().count(), 1, "{ran:?}");
    let line = ran.out.lines().next().unwrap_or_default();
    assert!(line.starts_with("chert:"), "{line}");
    assert!(line.contains(&format!("{SESSION} {NOW}")), "{line}");
    Ok(())
}

#[test]
fn search_with_no_hit_is_exit_0_and_empty() -> Test {
    let s = stored("search-none")?;
    let ran = s.run(&["search", "zebra", "--no-semantic"]);
    assert_eq!((ran.code, ran.out.as_str()), (0, ""), "{ran:?}");
    Ok(())
}

#[test]
fn search_since_and_project_narrow_the_rows() -> Test {
    let s = stored("search-narrow")?;
    let after = "1792800000001"; // NOW + 1 ms
    let ran = s.run(&["search", "chert-fixture", "--since", after]);
    assert_eq!((ran.code, ran.out.as_str()), (0, ""), "{ran:?}");
    let ran = s.run(&["search", "chert-fixture", "--since", "1d"]);
    assert_eq!((ran.code, ran.out.lines().count()), (0, 1), "{ran:?}");
    let ran = s.run(&["search", "chert-fixture", "--project", "none-0"]);
    assert_eq!((ran.code, ran.out.as_str()), (0, ""), "{ran:?}");
    Ok(())
}

#[test]
fn search_with_a_bad_since_is_exit_2() -> Test {
    let s = Sandbox::new("search-bad-since")?;
    let ran = s.run(&["search", "x", "--since", "soon"]);
    assert_eq!(ran.code, 2, "{ran:?}");
    Ok(())
}

#[test]
fn timeline_around_a_row_is_exit_0() -> Test {
    let s = stored("timeline-around")?;
    let reference = first_ref(&s.run(&["timeline", SESSION]).out)?;
    let ran = s.run(&["timeline", SESSION, "--around", &reference]);
    assert_eq!(ran.code, 0, "{ran:?}");
    let want = format!("{reference} post-tool-use {NOW}\n");
    assert_eq!(ran.out, want);
    Ok(())
}

#[test]
fn timeline_of_an_unknown_session_is_exit_0_and_empty() -> Test {
    let s = stored("timeline-none")?;
    let ran = s.run(&["timeline", "no-such-session"]);
    assert_eq!((ran.code, ran.out.as_str()), (0, ""), "{ran:?}");
    Ok(())
}

#[test]
fn timeline_around_a_dropped_ref_is_exit_1() -> Test {
    let s = stored("timeline-dropped")?;
    let ran = s.run(&["timeline", SESSION, "--around", "chert:ZZZZZZZZ"]);
    assert_eq!(ran.code, 1, "{ran:?}");
    assert!(ran.err.contains("dropped"), "{ran:?}");
    Ok(())
}

#[test]
fn timeline_around_a_bad_ref_is_exit_2() -> Test {
    let s = stored("timeline-bad-ref")?;
    let ran = s.run(&["timeline", SESSION, "--around", "01ABC"]);
    assert_eq!(ran.code, 2, "{ran:?}");
    Ok(())
}

#[test]
fn get_prints_the_row_and_its_body_with_exit_0() -> Test {
    let s = stored("get-found")?;
    let reference = first_ref(&s.run(&["timeline", SESSION]).out)?;
    let ran = s.run(&["get", &reference]);
    assert_eq!(ran.code, 0, "{ran:?}");
    let head = format!("{reference} post-tool-use {NOW} {SESSION}");
    assert!(ran.out.starts_with(&head), "{ran:?}");
    assert!(ran.out.contains("chert-fixture"), "{ran:?}");
    Ok(())
}

#[test]
fn get_naming_a_dropped_ref_is_exit_1() -> Test {
    let s = stored("get-dropped")?;
    let reference = first_ref(&s.run(&["timeline", SESSION]).out)?;
    let ran = s.run(&["get", &reference, "chert:ZZZZZZZZ"]);
    assert_eq!(ran.code, 1, "{ran:?}");
    assert!(ran.out.ends_with("chert:ZZZZZZZZ dropped\n"), "{ran:?}");
    Ok(())
}

#[test]
fn get_with_a_bad_ref_or_none_is_exit_2() -> Test {
    let s = Sandbox::new("get-bad")?;
    let ran = s.run(&["get", "chert:lower"]);
    assert_eq!(ran.code, 2, "{ran:?}");
    assert!(ran.err.starts_with("chert: "), "{ran:?}");
    assert_eq!(s.run(&["get"]).code, 2);
    Ok(())
}
