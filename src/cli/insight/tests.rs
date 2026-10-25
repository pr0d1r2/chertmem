//! `chert mine` is exit 1 with findings, 0 without; `chert report` is
//! exit 0 in every format (V2).

use super::super::tests::{Res, Sandbox, Test};

/// A sandbox whose 1 Bash call is already a finding.
fn mined(name: &str) -> Res<Sandbox> {
    let s = Sandbox::new(name)?;
    s.project_toml("[mining]\nmin_occurrences = 1\n")?;
    s.hook("post-tool-use")?;
    Ok(s)
}

#[test]
fn mine_without_findings_is_exit_0() -> Test {
    let s = Sandbox::new("mine-none")?;
    let ran = s.run(&["mine"]);
    assert_eq!((ran.code, ran.out.as_str()), (0, "chert: no findings\n"));
    Ok(())
}

#[test]
fn mine_with_findings_is_exit_1() -> Test {
    let s = mined("mine-found")?;
    let ran = s.run(&["mine", "--since", "1d"]);
    assert_eq!(ran.code, 1, "{ran:?}");
    assert!(ran.out.contains("chert:"), "{ran:?}");
    Ok(())
}

#[test]
fn mine_json_is_1_array() -> Test {
    let s = mined("mine-json")?;
    let ran = s.run(&["mine", "--format", "json"]);
    assert_eq!(ran.code, 1, "{ran:?}");
    assert!(ran.out.starts_with("[{"), "{ran:?}");
    Ok(())
}

#[test]
fn mine_refuses_md_and_a_bad_since() -> Test {
    let s = Sandbox::new("mine-bad")?;
    assert_eq!(s.run(&["mine", "--format", "md"]).code, 2);
    assert_eq!(s.run(&["mine", "--since", "-1d"]).code, 2);
    Ok(())
}

#[test]
fn report_is_exit_0_in_every_format() -> Test {
    let s = mined("report")?;
    for format in ["text", "json", "md"] {
        let ran = s.run(&["report", "--format", format]);
        assert_eq!(ran.code, 0, "{format}: {ran:?}");
        assert!(ran.out.contains("chert:"), "{format}: {ran:?}");
    }
    assert_eq!(s.run(&["report"]).code, 0);
    Ok(())
}

#[test]
fn mine_with_no_data_dir_is_exit_2() -> Test {
    let s = Sandbox::new("mine-no-data")?;
    assert_eq!(s.run_homeless(&["report"]).code, 2);
    Ok(())
}
