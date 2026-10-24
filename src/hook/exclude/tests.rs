use std::path::Path;

use super::{excluded, matches};
use crate::hook::tests::{Scratch, Test, fixture};
use crate::hook::{Kind, Outcome};

const ROOT: &str = "/workspace/project";

fn hit(glob: &str, path: &str) -> bool {
    let globs = [glob.to_owned()];
    excluded(&globs, Path::new(ROOT), &[path.to_owned()])
}

#[test]
fn a_glob_without_slash_matches_any_component() {
    assert!(hit(".env", "/workspace/project/.env"));
    assert!(hit(".env", "/workspace/project/app/.env"));
    assert!(hit("*.pem", "/workspace/project/keys/server.pem"));
    assert!(hit("secrets", "/workspace/project/secrets/db.toml"));
    assert!(!hit(".env", "/workspace/project/.envrc"));
    assert!(!hit("*.pem", "/workspace/project/pem.txt"));
}

#[test]
fn a_glob_with_slash_matches_from_the_project_root() {
    assert!(hit("secrets/**", "/workspace/project/secrets/a/b.txt"));
    assert!(hit(
        "config/prod.toml",
        "/workspace/project/config/prod.toml"
    ));
    assert!(hit("config/prod.toml", "config/prod.toml"));
    assert!(hit("config/prod.toml", "./config/prod.toml"));
    assert!(!hit(
        "config/prod.toml",
        "/workspace/project/x/config/prod.toml"
    ));
}

#[test]
fn a_matched_directory_excludes_what_is_under_it() {
    assert!(hit("vault/keys", "/workspace/project/vault/keys/id_rsa"));
    assert!(!hit("vault/keys", "/workspace/project/vault/keyring"));
}

#[test]
fn an_absolute_glob_matches_paths_outside_the_project() {
    assert!(hit("/etc/**", "/etc/ssl/private.key"));
    assert!(!hit("/etc/**", "/workspace/project/etc/a"));
}

#[test]
fn star_stays_in_its_component_and_double_star_does_not() {
    assert!(matches("src/*.rs", "src/a.rs"));
    assert!(!matches("src/*.rs", "src/a/b.rs"));
    assert!(matches("src/**/*.rs", "src/a/b.rs"));
    assert!(matches("src/**/*.rs", "src/b.rs"));
    assert!(matches("**/id_*", "home/.ssh/id_ed25519"));
    assert!(matches("**/id_*", "id_rsa"));
}

#[test]
fn question_mark_is_one_character_but_never_a_slash() {
    assert!(matches("key?.txt", "key1.txt"));
    assert!(!matches("key?.txt", "key/.txt"));
    assert!(!matches("key?.txt", "key.txt"));
}

#[test]
fn no_glob_or_no_path_excludes_nothing() {
    let root = Path::new(ROOT);
    assert!(!excluded(
        &[],
        root,
        &["/workspace/project/.env".to_owned()]
    ));
    assert!(!excluded(&[".env".to_owned()], root, &[]));
}

#[test]
fn an_excluded_path_stores_nothing() -> Test {
    let mut scratch = Scratch::new("excluded")?;
    scratch.config.privacy.exclude = vec!["src/*.rs".to_owned()];
    let path = scratch.root().join("src/main.rs");
    let text = serde_json::json!({
        "session_id": "s",
        "tool_name": "Read",
        "tool_input": {"file_path": path.to_string_lossy()},
        "tool_response": {"content": "fn main() {}"},
    });
    let out = scratch.run(Kind::PostToolUse, text.to_string().as_bytes());
    let none = Outcome {
        stdout: None,
        exit: 0,
        note: None,
    };
    assert_eq!(out, none);
    assert!(scratch.observations()?.is_empty());
    let kind = Kind::PostToolUse;
    scratch.run(kind, &fixture(kind)?);
    assert_eq!(scratch.observations()?.len(), 1);
    Ok(())
}
