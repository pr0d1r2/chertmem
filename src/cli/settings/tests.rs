//! `chert config show|get|set|path`: exit 0; an unknown key, a bad
//! value or no home for `--user` is exit 2 (V2).

use std::fs;

use super::super::tests::{Sandbox, Test};

#[test]
fn show_lists_each_key_with_its_origin() -> Test {
    let s = Sandbox::new("config-show")?;
    s.project_toml("[search]\ndefault_limit = 4\n")?;
    let ran = s.run(&["config", "show"]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert!(ran.out.contains("cache.window = \"28d\"  # default\n"));
    let project = s.project().join("chert.toml");
    let line =
        format!("search.default_limit = 4  # project {}", project.display());
    assert!(ran.out.contains(&line), "{ran:?}");
    Ok(())
}

#[test]
fn show_names_the_user_file_and_an_unset_data_dir() -> Test {
    let s = Sandbox::new("config-show-user")?;
    let user = s.home().join(".config/chert");
    fs::create_dir_all(&user)?;
    fs::write(user.join("chert.toml"), "[mining]\nenabled = false\n")?;
    let ran = s.run(&["config", "show"]);
    assert!(
        ran.out.contains("mining.enabled = false  # user "),
        "{ran:?}"
    );
    let homeless = s.run_homeless(&["config", "show"]);
    assert!(homeless.out.contains("storage.data_dir = (unset)"));
    Ok(())
}

#[test]
fn get_prints_1_value_and_refuses_an_unknown_key() -> Test {
    let s = Sandbox::new("config-get")?;
    let ran = s.run(&["config", "get", "inject.max_tokens"]);
    assert_eq!((ran.code, ran.out.as_str()), (0, "300\n"), "{ran:?}");
    let homeless = s.run_homeless(&["config", "get", "storage.data_dir"]);
    assert_eq!(homeless.out, "(unset)\n");
    let bad = s.run(&["config", "get", "cache.size"]);
    assert_eq!(bad.code, 2, "{bad:?}");
    assert!(bad.err.contains("unknown key cache.size"), "{bad:?}");
    Ok(())
}

#[test]
fn set_writes_the_project_file_then_the_value_reads_back() -> Test {
    let s = Sandbox::new("config-set")?;
    let ran = s.run(&["config", "set", "search.default_limit", "7"]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert!(ran.out.starts_with("set search.default_limit in "));
    let text = fs::read_to_string(s.project().join("chert.toml"))?;
    assert!(text.contains("default_limit = 7"), "{text}");
    let get = s.run(&["config", "get", "search.default_limit"]);
    assert_eq!(get.out, "7\n");
    Ok(())
}

#[test]
fn set_user_writes_the_user_file_and_says_the_disclaimer() -> Test {
    let s = Sandbox::new("config-set-user")?;
    let args = ["config", "set", "inject.on_prompt", "true", "--user"];
    let ran = s.run(&args);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert_eq!(ran.out.lines().count(), 2, "{ran:?}");
    assert!(s.home().join(".config/chert/chert.toml").exists());
    assert!(!s.project().join("chert.toml").exists());
    Ok(())
}

#[test]
fn set_refuses_a_bad_value_and_a_missing_home() -> Test {
    let s = Sandbox::new("config-set-bad")?;
    let ran = s.run(&["config", "set", "search.alpha", "2"]);
    assert_eq!(ran.code, 2, "{ran:?}");
    assert!(!s.project().join("chert.toml").exists());
    let user = ["config", "set", "search.alpha", "0.3", "--user"];
    let homeless = s.run_homeless(&user);
    assert_eq!(homeless.code, 2, "{homeless:?}");
    assert!(homeless.err.contains("HOME"), "{homeless:?}");
    Ok(())
}

#[test]
fn path_prints_both_files() -> Test {
    let s = Sandbox::new("config-path")?;
    let ran = s.run(&["config", "path"]);
    assert_eq!(ran.code, 0, "{ran:?}");
    let user = s.home().join(".config/chert/chert.toml");
    let project = s.project().join("chert.toml");
    let want =
        format!("user {}\nproject {}\n", user.display(), project.display());
    assert_eq!(ran.out, want);
    let homeless = s.run_homeless(&["config", "path"]);
    assert!(homeless.out.starts_with("user (no home)\n"), "{homeless:?}");
    Ok(())
}
