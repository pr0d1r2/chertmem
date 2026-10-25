use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use super::set;
use crate::config::tests::{Scratch, Test, put};
use crate::config::{Error as ConfigError, ON_PROMPT_DISCLAIMER};

/// The text of `file` after setting `key` to `value` in one holding
/// `before`.
fn after(
    name: &str,
    before: &str,
    key: &str,
    value: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let s = Scratch::new(name)?;
    put(&s.project_file(), before)?;
    set(&s.project_file(), key, value)?;
    Ok(fs::read_to_string(s.project_file())?)
}

#[test]
fn set_creates_an_absent_file_and_its_directory() -> Test {
    let s = Scratch::new("setcreate")?;
    assert_eq!(set(&s.user_file(), "cache.window", "\"14d\"")?, None);
    let text = fs::read_to_string(s.user_file())?;
    assert_eq!(text, "[cache]\nwindow = \"14d\"\n");
    let shown = s.load()?.config.cache.window;
    assert_eq!(shown, crate::config::tests::days(14));
    Ok(())
}

const BEFORE: &str = "# mine\n[cache]\nwindow = \"28d\"   # long\n\
    max_bytes = \"1GiB\"\n\n[search]\nalpha = 0.2\n";

#[test]
fn set_keeps_every_other_byte() -> Test {
    let text = after("setkeep", BEFORE, "cache.window", "14d")?;
    let want = BEFORE.replace("\"28d\"", "\"14d\"");
    assert_eq!(text, want);
    Ok(())
}

#[test]
fn set_adds_a_key_to_its_section() -> Test {
    let text = after("setadd", BEFORE, "cache.on_overflow", "fail")?;
    let want =
        BEFORE.replace("\"1GiB\"\n", "\"1GiB\"\non_overflow = \"fail\"\n");
    assert_eq!(text, want);
    Ok(())
}

#[test]
fn set_reaches_into_the_zstd_table() -> Test {
    let text = after("setzstd", "", "compression.zstd.level", "5")?;
    assert_eq!(text, "[compression.zstd]\nlevel = 5\n");
    let inline = "compression.zstd = { dictionary = false }\n";
    let text = after("setinline", inline, "compression.zstd.level", "5")?;
    let want = "compression.zstd = { dictionary = false , level = 5 }\n";
    assert_eq!(text, want);
    Ok(())
}

/// `set` on a file holding `before` fails with `want` and leaves it be.
fn refused(
    file: &Path,
    before: &str,
    (key, value): (&str, &str),
) -> Result<ConfigError, Box<dyn std::error::Error>> {
    put(file, before)?;
    let Err(e) = set(file, key, value) else {
        return Err(format!("{key} = {value} was set").into());
    };
    assert_eq!(fs::read_to_string(file)?, before);
    Ok(e)
}

#[test]
fn set_refuses_an_unknown_key() -> Test {
    let s = Scratch::new("setunknown")?;
    let file = s.project_file();
    let e = refused(&file, BEFORE, ("cache.size", "1"))?;
    let key = "cache.size".to_owned();
    assert_eq!(e, ConfigError::UnknownKey { file, key });
    Ok(())
}

#[test]
fn set_refuses_an_invalid_value_as_load_does() -> Test {
    let s = Scratch::new("setinvalid")?;
    let file = s.project_file();
    let e = refused(&file, BEFORE, ("cache.window", "4w"))?;
    let ConfigError::Invalid { key, value, .. } = &e else {
        return Err(format!("{e:?}").into());
    };
    assert_eq!((key.as_str(), value.as_str()), ("cache.window", "\"4w\""));
    assert_eq!(e.file(), file);
    Ok(())
}

#[test]
fn set_refuses_a_file_that_does_not_load() -> Test {
    let s = Scratch::new("setbroken")?;
    let file = s.project_file();
    let e = refused(&file, "cache = 1\n", ("cache.window", "7d"))?;
    let key = "cache".to_owned();
    assert_eq!(e, ConfigError::UnknownKey { file, key });
    let e = refused(&s.project_file(), "[cache\n", ("cache.window", "7d"))?;
    assert!(matches!(e, ConfigError::Parse { .. }), "{e:?}");
    Ok(())
}

#[test]
fn turning_on_prompt_on_hands_back_the_disclaimer() -> Test {
    let s = Scratch::new("setdisclaimer")?;
    let file = s.project_file();
    let on = set(&file, "inject.on_prompt", "true")?;
    assert_eq!(on, Some(ON_PROMPT_DISCLAIMER));
    assert_eq!(set(&file, "inject.on_prompt", "true")?, None);
    assert_eq!(set(&file, "inject.on_prompt", "false")?, None);
    assert_eq!(set(&file, "inject.max_tokens", "200")?, None);
    Ok(())
}

#[test]
fn an_unreadable_or_unwritable_file_is_an_error() -> Test {
    let s = Scratch::new("setio")?;
    let dir = s.project_file();
    fs::create_dir_all(&dir)?;
    let e = set(&dir, "cache.window", "7d");
    assert!(matches!(e, Err(ConfigError::Read { .. })), "{e:?}");
    let locked = s.home().join("locked");
    fs::create_dir_all(&locked)?;
    fs::set_permissions(&locked, Permissions::from_mode(0o555))?;
    let e = set(&locked.join("chert.toml"), "cache.window", "7d");
    fs::set_permissions(&locked, Permissions::from_mode(0o755))?;
    assert!(matches!(e, Err(ConfigError::Write { .. })), "{e:?}");
    Ok(())
}
