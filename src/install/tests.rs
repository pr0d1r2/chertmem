//! Install and uninstall on temp config files (src/install V1, V2),
//! and the scratch dir every install test uses.

use std::error::Error as StdError;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{Change, Error, Target, install, settings_path, uninstall};
use crate::hook::Ide;

pub(super) type Test = Result<(), Box<dyn StdError>>;
pub(super) type Res<T> = Result<T, Box<dyn StdError>>;

/// The `chert` every test wires.
pub(super) const CHERT: &str = "/opt/chert/bin/chert";

/// A settings file as a person keeps it: 4-space indent, keys out of
/// order, no final newline, a hook and an MCP server of their own.
pub(super) const ORIGINAL: &str = r#"{
    "theme": "dark",
    "hooks": {
        "PostToolUse": [
            {"matcher": "Bash", "hooks": [{"type": "command", "command": "lint.sh"}]}
        ]
    },
    "mcpServers": {"other": {"command": "other-mcp"}},
    "env": {"B": "2", "A": "1"}
}"#;

/// A scratch home dir, removed on drop.
pub(super) struct Scratch {
    pub(super) dir: PathBuf,
}

impl Scratch {
    pub(super) fn new(name: &str) -> Res<Self> {
        let pid = std::process::id();
        let dir =
            std::env::temp_dir().join(format!("chert-install-{pid}-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    /// `<home>/.claude/settings.json`, not created.
    pub(super) fn settings(&self) -> PathBuf {
        settings_path(&self.dir, Ide::ClaudeCode)
    }

    /// The settings file holding `text`.
    pub(super) fn with(&self, text: &str) -> Res<PathBuf> {
        let path = self.settings();
        fs::create_dir_all(path.parent().ok_or("no parent")?)?;
        fs::write(&path, text)?;
        Ok(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

pub(super) fn target(path: &Path) -> Target<'_> {
    Target {
        settings: path,
        chert: CHERT,
    }
}

pub(super) fn json_at(path: &Path) -> Res<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn bak(path: &Path) -> PathBuf {
    path.with_file_name("settings.json.chert.bak")
}

#[test]
fn settings_path_is_claude_codes_user_settings() {
    let got = settings_path(Path::new("home"), Ide::ClaudeCode);
    assert_eq!(got, Path::new("home/.claude/settings.json"));
}

#[test]
fn install_then_uninstall_gives_back_the_same_bytes() -> Test {
    let s = Scratch::new("v1-bytes")?;
    let path = s.with(ORIGINAL)?;
    assert_eq!(install(&target(&path), Ide::ClaudeCode)?, Change::Wrote);
    assert_ne!(fs::read_to_string(&path)?, ORIGINAL);
    assert_eq!(
        uninstall(&target(&path), Ide::ClaudeCode)?,
        Change::Restored
    );
    assert_eq!(fs::read_to_string(&path)?, ORIGINAL);
    assert!(!bak(&path).exists(), "backup left behind");
    Ok(())
}

#[test]
fn install_then_uninstall_without_a_file_leaves_no_file() -> Test {
    let s = Scratch::new("v1-absent")?;
    let path = s.settings();
    assert_eq!(install(&target(&path), Ide::ClaudeCode)?, Change::Wrote);
    assert!(path.exists() && !bak(&path).exists());
    assert_eq!(
        uninstall(&target(&path), Ide::ClaudeCode)?,
        Change::Restored
    );
    assert!(!path.exists(), "file chert created is still there");
    Ok(())
}

#[test]
fn install_then_uninstall_of_a_blank_file_gives_it_back() -> Test {
    let s = Scratch::new("v1-blank")?;
    let path = s.with("\n")?;
    install(&target(&path), Ide::ClaudeCode)?;
    assert_eq!(
        uninstall(&target(&path), Ide::ClaudeCode)?,
        Change::Restored
    );
    assert_eq!(fs::read_to_string(&path)?, "\n");
    Ok(())
}

#[test]
fn install_twice_is_install_once() -> Test {
    let s = Scratch::new("v2")?;
    let path = s.with(ORIGINAL)?;
    install(&target(&path), Ide::ClaudeCode)?;
    let once = fs::read(&path)?;
    assert_eq!(install(&target(&path), Ide::ClaudeCode)?, Change::Unchanged);
    assert_eq!(fs::read(&path)?, once);
    assert_eq!(fs::read_to_string(bak(&path))?, ORIGINAL);
    Ok(())
}

#[test]
fn the_backup_holds_the_bytes_before_the_first_write() -> Test {
    let s = Scratch::new("backup")?;
    let path = s.with(ORIGINAL)?;
    install(&target(&path), Ide::ClaudeCode)?;
    let other = Target {
        chert: "/elsewhere/chert",
        ..target(&path)
    };
    assert_eq!(install(&other, Ide::ClaudeCode)?, Change::Wrote);
    assert_eq!(fs::read_to_string(bak(&path))?, ORIGINAL);
    Ok(())
}

#[test]
fn install_keeps_every_unrelated_key() -> Test {
    let s = Scratch::new("keeps")?;
    let path = s.with(ORIGINAL)?;
    install(&target(&path), Ide::ClaudeCode)?;
    let got = json_at(&path)?;
    assert_eq!(at(&got, "/theme"), "dark");
    assert_eq!(at(&got, "/env"), json!({"A": "1", "B": "2"}));
    let other = json!({"command": "other-mcp"});
    assert_eq!(at(&got, "/mcpServers/other"), other);
    let user = at(&got, "/hooks/PostToolUse/0");
    assert_eq!(at(&user, "/matcher"), "Bash");
    assert_eq!(at(&user, "/hooks/0/command"), "lint.sh");
    Ok(())
}

/// The value at JSON pointer `ptr`, null when there is none.
pub(super) fn at(value: &Value, ptr: &str) -> Value {
    value.pointer(ptr).cloned().unwrap_or_default()
}

/// Claude Code's event names and chert's.
const EVENTS: [(&str, &str); 5] = [
    ("SessionStart", "session-start"),
    ("UserPromptSubmit", "user-prompt-submit"),
    ("PostToolUse", "post-tool-use"),
    ("Stop", "stop"),
    ("SessionEnd", "session-end"),
];

#[test]
fn install_writes_5_hooks_and_the_mcp_server() -> Test {
    let s = Scratch::new("writes")?;
    let path = s.settings();
    install(&target(&path), Ide::ClaudeCode)?;
    let got = json_at(&path)?;
    for (name, event) in EVENTS {
        let cmd = format!("{CHERT} hook run {event} --ide claude-code");
        let hook = json!({"type": "command", "command": cmd});
        let want = json!([{"hooks": [hook]}]);
        assert_eq!(at(&got, &format!("/hooks/{name}")), want, "{name}");
    }
    let server = json!({"command": CHERT, "args": ["mcp"]});
    assert_eq!(at(&got, "/mcpServers"), json!({"chert": server}));
    Ok(())
}

#[test]
fn uninstall_keeps_an_edit_made_since_install() -> Test {
    let s = Scratch::new("edited")?;
    let path = s.with(ORIGINAL)?;
    install(&target(&path), Ide::ClaudeCode)?;
    let model = |mut v: Value| {
        v.as_object_mut()
            .map(|o| o.insert("model".into(), json!("opus")));
        v
    };
    fs::write(&path, serde_json::to_vec(&model(json_at(&path)?))?)?;
    assert_eq!(uninstall(&target(&path), Ide::ClaudeCode)?, Change::Wrote);
    let want = model(serde_json::from_str(ORIGINAL)?);
    assert_eq!(json_at(&path)?, want);
    assert!(bak(&path).exists(), "backup dropped though bytes differ");
    Ok(())
}

#[test]
fn uninstall_of_a_file_without_chert_is_unchanged() -> Test {
    let s = Scratch::new("none")?;
    let path = s.with(ORIGINAL)?;
    assert_eq!(
        uninstall(&target(&path), Ide::ClaudeCode)?,
        Change::Unchanged
    );
    assert_eq!(fs::read_to_string(&path)?, ORIGINAL);
    let absent = s.dir.join("absent.json");
    assert_eq!(
        uninstall(&target(&absent), Ide::ClaudeCode)?,
        Change::Unchanged
    );
    assert!(!absent.exists());
    Ok(())
}

#[test]
fn install_with_another_chert_replaces_the_old_entries() -> Test {
    let s = Scratch::new("moved")?;
    let path = s.settings();
    install(&target(&path), Ide::ClaudeCode)?;
    let moved = Target {
        chert: "/new place/chert",
        ..target(&path)
    };
    install(&moved, Ide::ClaudeCode)?;
    let got = json_at(&path)?;
    let cmd = "'/new place/chert' hook run stop --ide claude-code";
    let hook = json!({"type": "command", "command": cmd});
    assert_eq!(at(&got, "/hooks/Stop"), json!([{"hooks": [hook]}]));
    let server = at(&got, "/mcpServers/chert/command");
    assert_eq!(server, "/new place/chert");
    Ok(())
}

#[test]
fn a_file_that_is_not_json_is_an_error_and_left_alone() -> Test {
    let s = Scratch::new("not-json")?;
    let path = s.with("{ not json")?;
    let got = install(&target(&path), Ide::ClaudeCode);
    assert!(matches!(got, Err(Error::Json { .. })), "{got:?}");
    assert_eq!(fs::read_to_string(&path)?, "{ not json");
    assert!(!bak(&path).exists());
    Ok(())
}

/// Settings with a wrong-typed key chert edits, and that key.
const SHAPES: [(&str, &str); 4] = [
    (r#"{"hooks": []}"#, "hooks"),
    (r#"{"hooks": {"Stop": {}}}"#, "Stop"),
    (r#"{"mcpServers": 1}"#, "mcpServers"),
    ("[]", "settings"),
];

#[test]
fn a_wrong_typed_key_is_an_error_naming_it() -> Test {
    let s = Scratch::new("shape")?;
    for (text, key) in SHAPES {
        let path = s.with(text)?;
        let got = install(&target(&path), Ide::ClaudeCode);
        let named =
            matches!(&got, Err(Error::Shape { key: k, .. }) if k == key);
        assert!(named, "{text}: {got:?}");
        // uninstall only reads `mcpServers` when it is an object.
        let got = uninstall(&target(&path), Ide::ClaudeCode);
        assert_eq!(got.is_err(), key != "mcpServers", "{text}: {got:?}");
    }
    Ok(())
}
