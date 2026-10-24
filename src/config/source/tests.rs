use std::fs;

use crate::config::Error as ConfigError;
use crate::config::tests::{Scratch, Test, project_error, put};

#[test]
fn unknown_key_names_key_and_file() -> Test {
    let s = Scratch::new("unknown")?;
    put(&s.user_file(), "[cache]\nwindoww = \"7d\"\n")?;
    let want = ConfigError::UnknownKey {
        file: s.user_file(),
        key: "cache.windoww".to_owned(),
    };
    assert_eq!(s.load(), Err(want));
    Ok(())
}

const UNKNOWN: &[(&str, &str)] = &[
    ("colour = 1\n", "colour"),
    ("[nope]\na = 1\n", "nope.a"),
    ("cache = 1\n", "cache"),
];

#[test]
fn unknown_sections_are_errors() -> Test {
    for (i, &(text, key)) in UNKNOWN.iter().enumerate() {
        let e = project_error(&format!("section{i}"), text)?;
        let want = ConfigError::UnknownKey {
            file: e.file().to_path_buf(),
            key: key.to_owned(),
        };
        assert_eq!(e, want, "{text:?}");
    }
    Ok(())
}

#[test]
fn unknown_zstd_key_is_an_unknown_key() -> Test {
    let text = "compression.zstd = { window = 3 }\n";
    let e = project_error("zstdunknown", text)?;
    let want = ConfigError::UnknownKey {
        file: e.file().to_path_buf(),
        key: "compression.zstd.window".to_owned(),
    };
    assert_eq!(e, want);
    Ok(())
}

#[test]
fn table_where_value_belongs_is_invalid() -> Test {
    let e = project_error("tablevalue", "[cache.window]\na = 1\n")?;
    let invalid = matches!(e, ConfigError::Invalid { ref key, .. } if key == "cache.window");
    assert!(invalid, "{e}");
    Ok(())
}

#[test]
fn invalid_value_names_file_key_value_and_forms() -> Test {
    let s = Scratch::new("invalid")?;
    put(&s.project_file(), "[cache]\nwindow = \"4w\"\n")?;
    let want = ConfigError::Invalid {
        file: s.project_file(),
        key: "cache.window".to_owned(),
        value: "\"4w\"".to_owned(),
        accepted: "<n>d, <n>h or <n>mo with n >= 1".to_owned(),
    };
    assert_eq!(s.load(), Err(want));
    Ok(())
}

#[test]
fn broken_toml_is_a_parse_error() -> Test {
    let e = project_error("parse", "[cache\n")?;
    assert!(matches!(e, ConfigError::Parse { .. }), "{e}");
    Ok(())
}

#[test]
fn unreadable_file_is_a_read_error() -> Test {
    let s = Scratch::new("read")?;
    fs::create_dir_all(s.project_file())?;
    let e = s.load();
    assert!(matches!(e, Err(ConfigError::Read { .. })), "{e:?}");
    Ok(())
}
