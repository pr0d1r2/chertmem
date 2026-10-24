use std::path::PathBuf;

use super::Error as ConfigError;

fn invalid_window() -> ConfigError {
    ConfigError::Invalid {
        file: PathBuf::from("/p/chert.toml"),
        key: "cache.window".into(),
        value: "\"4w\"".into(),
        accepted: "<n>d".into(),
    }
}

fn every_error() -> [ConfigError; 4] {
    let p = || PathBuf::from("/p/chert.toml");
    let (file, reason, key) = (p(), "denied".to_owned(), "a.b".to_owned());
    let read = ConfigError::Read { file, reason };
    let (file, reason) = (p(), "line 1".to_owned());
    let parse = ConfigError::Parse { file, reason };
    let file = p();
    let unknown = ConfigError::UnknownKey { file, key };
    [read, parse, unknown, invalid_window()]
}

#[test]
fn messages_follow_chert_what_fix() {
    for e in every_error() {
        let text = e.to_string();
        assert!(text.starts_with("chert: "), "{text}");
        assert!(
            text.contains("/p/chert.toml") && text.ends_with(')'),
            "{text}"
        );
    }
}

#[test]
fn invalid_message_names_key_value_and_forms() {
    let text = invalid_window().to_string();
    assert!(text.contains("cache.window = \"4w\""), "{text}");
    assert!(text.contains("accepted: <n>d"), "{text}");
}
