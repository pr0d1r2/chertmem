//! `chert install`, `uninstall`, `status`, `doctor`: the sandbox home's
//! IDE files only; exit 0, `doctor` 1 on findings, 2 without a home or
//! with an unreadable config file (V2).

use std::fs;

use super::super::tests::{CHERT, Sandbox, Test};

#[test]
fn install_writes_both_files_then_leaves_them() -> Test {
    let s = Sandbox::new("install")?;
    let ran = s.run(&["install"]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert!(ran.out.contains("settings.json: wrote"), "{ran:?}");
    assert!(ran.out.contains(".claude.json: wrote"), "{ran:?}");
    let settings = fs::read_to_string(s.home().join(".claude/settings.json"))?;
    assert!(settings.contains(CHERT), "{settings}");
    let again = s.run(&["install", "--ide", "claude-code"]);
    assert_eq!(again.code, 0, "{again:?}");
    assert_eq!(again.out.matches(": unchanged").count(), 2, "{again:?}");
    Ok(())
}

#[test]
fn uninstall_restores_the_files_chert_created() -> Test {
    let s = Sandbox::new("uninstall")?;
    assert_eq!(s.run(&["install"]).code, 0);
    let ran = s.run(&["uninstall"]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert_eq!(ran.out.matches(": restored").count(), 2, "{ran:?}");
    assert!(!s.home().join(".claude.json").exists());
    Ok(())
}

#[test]
fn status_names_each_entry_with_exit_0_either_way() -> Test {
    let s = Sandbox::new("status")?;
    let before = s.run(&["status"]);
    assert_eq!(before.code, 0, "{before:?}");
    assert!(before.out.contains(": missing (run chert install)"));
    assert_eq!(s.run(&["install"]).code, 0);
    let after = s.run(&["status"]);
    assert_eq!(after.code, 0, "{after:?}");
    assert!(after.out.lines().all(|l| l.ends_with(": ok")), "{after:?}");
    Ok(())
}

#[test]
fn status_names_an_entry_running_another_chert() -> Test {
    let s = Sandbox::new("status-other")?;
    assert_eq!(s.run(&["install"]).code, 0);
    let file = s.home().join(".claude.json");
    let text = fs::read_to_string(&file)?.replace(CHERT, "/old/chert");
    fs::write(&file, text)?;
    let ran = s.run(&["status"]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert!(ran.out.contains("runs "), "{ran:?}");
    Ok(())
}

#[test]
fn doctor_is_exit_1_on_findings_and_0_once_wired() -> Test {
    let s = Sandbox::new("doctor")?;
    let before = s.run(&["doctor"]);
    assert_eq!(before.code, 1, "{before:?}");
    assert!(before.out.contains("wiring "), "{before:?}");
    assert_eq!(s.run(&["install"]).code, 0);
    s.hook("post-tool-use")?;
    let after = s.run(&["doctor"]);
    assert_eq!(after.code, 0, "{after:?}");
    assert!(after.out.ends_with("chert: no findings\n"), "{after:?}");
    Ok(())
}

#[test]
fn doctor_prints_config_warnings() -> Test {
    let s = Sandbox::new("doctor-warn")?;
    s.project_toml("[cache]\nwindow = \"10d\"\n")?;
    assert_eq!(s.run(&["install"]).code, 0);
    let ran = s.run(&["doctor"]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert!(ran.out.contains("10d"), "{ran:?}");
    Ok(())
}

#[test]
fn every_wiring_verb_without_a_home_is_exit_2() -> Test {
    let s = Sandbox::new("no-home")?;
    for verb in ["install", "uninstall", "status", "doctor"] {
        let ran = s.run_homeless(&[verb]);
        assert_eq!(ran.code, 2, "{verb}: {ran:?}");
        assert!(ran.err.contains("HOME"), "{verb}: {ran:?}");
    }
    Ok(())
}

#[test]
fn a_config_file_that_is_not_json_is_exit_2() -> Test {
    let s = Sandbox::new("install-bad-json")?;
    fs::write(s.home().join(".claude.json"), "{ nope")?;
    for verb in ["install", "uninstall", "status", "doctor"] {
        assert_eq!(s.run(&[verb]).code, 2, "{verb}");
    }
    s.project_toml("[cache]\nwindow = 7\n")?;
    fs::remove_file(s.home().join(".claude.json"))?;
    assert_eq!(s.run(&["doctor"]).code, 2);
    Ok(())
}
