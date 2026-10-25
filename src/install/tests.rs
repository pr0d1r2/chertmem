//! Install and uninstall on temp config files (src/install V1, V2, V4),
//! and the scratch dir every install test uses.

use std::error::Error as StdError;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{Change, Changes, Error, Target, install, uninstall};
use super::{mcp_path, settings_path};
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

/// A `~/.claude.json` as Claude Code keeps it: compact, keys of its
/// own, a user-scope server of the person's.
pub(super) const CLAUDE_JSON: &str = r#"{"numStartups":7,"projects":{"/p":{"allowedTools":[]}},"mcpServers":{"other":{"type":"stdio","command":"other-mcp","args":[],"env":{}}}}"#;

/// A scratch home dir, removed on drop.
pub(super) struct Scratch {
    pub(super) dir: PathBuf,
    /// `<home>/.claude/settings.json`, not created.
    pub(super) settings: PathBuf,
    /// `<home>/.claude.json`, not created.
    pub(super) mcp: PathBuf,
}

impl Scratch {
    pub(super) fn new(name: &str) -> Res<Self> {
        let pid = std::process::id();
        let dir =
            std::env::temp_dir().join(format!("chert-install-{pid}-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir)?;
        let settings = settings_path(&dir, Ide::ClaudeCode);
        let mcp = mcp_path(&dir, Ide::ClaudeCode);
        Ok(Self { dir, settings, mcp })
    }

    /// The settings file holding `text`.
    pub(super) fn with(&self, text: &str) -> Res<PathBuf> {
        let path = self.settings.clone();
        fs::create_dir_all(path.parent().ok_or("no parent")?)?;
        fs::write(&path, text)?;
        Ok(path)
    }

    /// The MCP file holding `text`.
    pub(super) fn with_mcp(&self, text: &str) -> Res<PathBuf> {
        fs::write(&self.mcp, text)?;
        Ok(self.mcp.clone())
    }

    /// Both files of this home, wiring [`CHERT`].
    pub(super) fn target(&self) -> Target<'_> {
        Target {
            settings: &self.settings,
            mcp: &self.mcp,
            chert: CHERT,
        }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

pub(super) fn json_at(path: &Path) -> Res<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn bak(path: &Path) -> PathBuf {
    super::settings::backup(path)
}

const fn both(change: Change) -> Changes {
    Changes {
        settings: change,
        mcp: change,
    }
}

#[test]
fn the_paths_are_claude_codes_user_settings_and_claude_json() {
    let got = settings_path(Path::new("home"), Ide::ClaudeCode);
    assert_eq!(got, Path::new("home/.claude/settings.json"));
    let got = mcp_path(Path::new("home"), Ide::ClaudeCode);
    assert_eq!(got, Path::new("home/.claude.json"));
}

#[test]
fn install_then_uninstall_gives_back_the_same_bytes() -> Test {
    let s = Scratch::new("v1-bytes")?;
    let path = s.with(ORIGINAL)?;
    let mcp = s.with_mcp(CLAUDE_JSON)?;
    assert_eq!(install(&s.target(), Ide::ClaudeCode)?, both(Change::Wrote));
    assert_ne!(fs::read_to_string(&mcp)?, CLAUDE_JSON);
    let back = uninstall(&s.target(), Ide::ClaudeCode)?;
    assert_eq!(back, both(Change::Restored));
    restored(&path, ORIGINAL)?;
    restored(&mcp, CLAUDE_JSON)
}

/// `path` holds `text` again, and no backup is left beside it.
fn restored(path: &Path, text: &str) -> Test {
    assert_eq!(fs::read_to_string(path)?, text);
    assert!(!bak(path).exists(), "backup of {path:?} left behind");
    Ok(())
}

#[test]
fn install_then_uninstall_without_files_leaves_no_files() -> Test {
    let s = Scratch::new("v1-absent")?;
    assert_eq!(install(&s.target(), Ide::ClaudeCode)?, both(Change::Wrote));
    for path in [&s.settings, &s.mcp] {
        assert!(path.exists() && !bak(path).exists(), "{path:?}");
    }
    let back = uninstall(&s.target(), Ide::ClaudeCode)?;
    assert_eq!(back, both(Change::Restored));
    assert!(!s.settings.exists(), "settings chert created still there");
    assert!(!s.mcp.exists(), "claude.json chert created still there");
    Ok(())
}

#[test]
fn install_then_uninstall_of_a_blank_file_gives_it_back() -> Test {
    let s = Scratch::new("v1-blank")?;
    let path = s.with("\n")?;
    install(&s.target(), Ide::ClaudeCode)?;
    uninstall(&s.target(), Ide::ClaudeCode)?;
    assert_eq!(fs::read_to_string(&path)?, "\n");
    Ok(())
}

#[test]
fn install_twice_is_install_once() -> Test {
    let s = Scratch::new("v2")?;
    let path = s.with(ORIGINAL)?;
    let mcp = s.with_mcp(CLAUDE_JSON)?;
    install(&s.target(), Ide::ClaudeCode)?;
    let once = (fs::read(&path)?, fs::read(&mcp)?);
    let again = install(&s.target(), Ide::ClaudeCode)?;
    assert_eq!(again, both(Change::Unchanged));
    assert_eq!((fs::read(&path)?, fs::read(&mcp)?), once);
    assert_eq!(fs::read_to_string(bak(&path))?, ORIGINAL);
    assert_eq!(fs::read_to_string(bak(&mcp))?, CLAUDE_JSON);
    Ok(())
}

#[test]
fn the_backups_hold_the_bytes_before_the_first_write() -> Test {
    let s = Scratch::new("backup")?;
    let path = s.with(ORIGINAL)?;
    let mcp = s.with_mcp(CLAUDE_JSON)?;
    install(&s.target(), Ide::ClaudeCode)?;
    let other = Target {
        chert: "/elsewhere/chert",
        ..s.target()
    };
    assert_eq!(install(&other, Ide::ClaudeCode)?, both(Change::Wrote));
    assert_eq!(fs::read_to_string(bak(&path))?, ORIGINAL);
    assert_eq!(fs::read_to_string(bak(&mcp))?, CLAUDE_JSON);
    Ok(())
}

#[test]
fn install_keeps_every_unrelated_key() -> Test {
    let s = Scratch::new("keeps")?;
    let path = s.with(ORIGINAL)?;
    install(&s.target(), Ide::ClaudeCode)?;
    let got = json_at(&path)?;
    assert_eq!(at(&got, "/theme"), "dark");
    assert_eq!(at(&got, "/env"), json!({"A": "1", "B": "2"}));
    let other = json!({"command": "other-mcp"});
    assert_eq!(at(&got, "/mcpServers"), json!({"other": other}));
    let user = at(&got, "/hooks/PostToolUse/0");
    assert_eq!(at(&user, "/matcher"), "Bash");
    assert_eq!(at(&user, "/hooks/0/command"), "lint.sh");
    Ok(())
}

#[test]
fn install_keeps_every_key_claude_code_keeps() -> Test {
    let s = Scratch::new("keeps-mcp")?;
    let mcp = s.with_mcp(CLAUDE_JSON)?;
    install(&s.target(), Ide::ClaudeCode)?;
    let got = json_at(&mcp)?;
    assert_eq!(at(&got, "/numStartups"), 7);
    assert_eq!(at(&got, "/projects/~1p/allowedTools"), json!([]));
    assert_eq!(at(&got, "/mcpServers/other/command"), "other-mcp");
    Ok(())
}

/// The value at JSON pointer `ptr`, null when there is none.
pub(super) fn at(value: &Value, ptr: &str) -> Value {
    value.pointer(ptr).cloned().unwrap_or_default()
}

/// Claude Code's event names and chert's.
const EVENTS: [(&str, &str); 6] = [
    ("SessionStart", "session-start"),
    ("UserPromptSubmit", "user-prompt-submit"),
    ("PostToolUse", "post-tool-use"),
    ("PostToolUseFailure", "post-tool-use-failure"),
    ("Stop", "stop"),
    ("SessionEnd", "session-end"),
];

#[test]
fn install_writes_6_hooks_to_settings_and_no_server() -> Test {
    let s = Scratch::new("writes")?;
    install(&s.target(), Ide::ClaudeCode)?;
    let got = json_at(&s.settings)?;
    for (name, event) in EVENTS {
        let cmd = format!("{CHERT} hook run {event} --ide claude-code");
        let hook = json!({"type": "command", "command": cmd});
        let want = json!([{"hooks": [hook]}]);
        assert_eq!(at(&got, &format!("/hooks/{name}")), want, "{name}");
    }
    assert_eq!(at(&got, "/mcpServers"), Value::Null, "server in settings");
    Ok(())
}

#[test]
fn install_writes_the_server_to_claude_json() -> Test {
    let s = Scratch::new("writes-mcp")?;
    install(&s.target(), Ide::ClaudeCode)?;
    let server = json!({
        "type": "stdio", "command": CHERT, "args": ["mcp"], "env": {},
    });
    let got = json_at(&s.mcp)?;
    assert_eq!(got, json!({"mcpServers": {"chert": server}}));
    Ok(())
}

#[test]
fn install_moves_the_server_chert_once_wrote_to_settings() -> Test {
    let s = Scratch::new("migrate")?;
    let old = json!({"command": CHERT, "args": ["mcp"]});
    let other = json!({"command": "other-mcp"});
    let both = json!({"mcpServers": {"chert": old, "other": other}});
    let path = s.with(&both.to_string())?;
    install(&s.target(), Ide::ClaudeCode)?;
    assert_eq!(at(&json_at(&path)?, "/mcpServers"), json!({"other": other}));
    assert_eq!(at(&json_at(&s.mcp)?, "/mcpServers/chert/command"), CHERT);
    let alone = json!({"mcpServers": {"chert": old}});
    s.with(&alone.to_string())?;
    install(&s.target(), Ide::ClaudeCode)?;
    assert_eq!(at(&json_at(&path)?, "/mcpServers"), Value::Null);
    Ok(())
}

/// `path`'s JSON with key `model` set, as an edit made since install.
fn edit(path: &Path) -> Res<Value> {
    let mut value = json_at(path)?;
    let top = value.as_object_mut().ok_or("not an object")?;
    top.insert("model".into(), json!("opus"));
    fs::write(path, serde_json::to_vec(&value)?)?;
    Ok(value)
}

#[test]
fn uninstall_keeps_an_edit_made_since_install() -> Test {
    let s = Scratch::new("edited")?;
    let path = s.with(ORIGINAL)?;
    let mcp = s.with_mcp(CLAUDE_JSON)?;
    install(&s.target(), Ide::ClaudeCode)?;
    edit(&path)?;
    edit(&mcp)?;
    let back = uninstall(&s.target(), Ide::ClaudeCode)?;
    assert_eq!(back, both(Change::Wrote));
    edit_kept(&path, ORIGINAL)?;
    edit_kept(&mcp, CLAUDE_JSON)
}

/// `path` is `text` with the edit [`edit`] makes, its backup kept.
fn edit_kept(path: &Path, text: &str) -> Test {
    let mut want: Value = serde_json::from_str(text)?;
    let top = want.as_object_mut().ok_or("not an object")?;
    top.insert("model".into(), json!("opus"));
    assert_eq!(json_at(path)?, want, "{path:?}");
    assert!(bak(path).exists(), "backup dropped though bytes differ");
    Ok(())
}

#[test]
fn uninstall_of_files_without_chert_is_unchanged() -> Test {
    let s = Scratch::new("none")?;
    let path = s.with(ORIGINAL)?;
    let mcp = s.with_mcp(CLAUDE_JSON)?;
    let back = uninstall(&s.target(), Ide::ClaudeCode)?;
    assert_eq!(back, both(Change::Unchanged));
    assert_eq!(fs::read_to_string(&path)?, ORIGINAL);
    assert_eq!(fs::read_to_string(&mcp)?, CLAUDE_JSON);
    let absent = Scratch::new("none-absent")?;
    let back = uninstall(&absent.target(), Ide::ClaudeCode)?;
    assert_eq!(back, both(Change::Unchanged));
    assert!(!absent.settings.exists() && !absent.mcp.exists());
    Ok(())
}

#[test]
fn install_with_another_chert_replaces_the_old_entries() -> Test {
    let s = Scratch::new("moved")?;
    install(&s.target(), Ide::ClaudeCode)?;
    let moved = Target {
        chert: "/new place/chert",
        ..s.target()
    };
    install(&moved, Ide::ClaudeCode)?;
    let got = json_at(&s.settings)?;
    let cmd = "'/new place/chert' hook run stop --ide claude-code";
    let hook = json!({"type": "command", "command": cmd});
    assert_eq!(at(&got, "/hooks/Stop"), json!([{"hooks": [hook]}]));
    let server = at(&json_at(&s.mcp)?, "/mcpServers/chert/command");
    assert_eq!(server, "/new place/chert");
    Ok(())
}

#[test]
fn a_file_that_is_not_json_is_an_error_and_left_alone() -> Test {
    let s = Scratch::new("not-json")?;
    let path = s.with("{ not json")?;
    let got = install(&s.target(), Ide::ClaudeCode);
    assert!(matches!(got, Err(Error::Json { .. })), "{got:?}");
    assert_eq!(fs::read_to_string(&path)?, "{ not json");
    assert!(!bak(&path).exists());
    Ok(())
}

#[test]
fn a_claude_json_that_is_not_json_stops_before_any_write() -> Test {
    let s = Scratch::new("mcp-not-json")?;
    let mcp = s.with_mcp("{ not json")?;
    for got in [
        install(&s.target(), Ide::ClaudeCode),
        uninstall(&s.target(), Ide::ClaudeCode),
    ] {
        assert!(matches!(got, Err(Error::Json { .. })), "{got:?}");
    }
    assert!(!s.settings.exists(), "settings written");
    assert_eq!(fs::read_to_string(&mcp)?, "{ not json");
    Ok(())
}

/// Settings with a wrong-typed key chert edits, and that key.
const SHAPES: [(&str, &str); 3] = [
    (r#"{"hooks": []}"#, "hooks"),
    (r#"{"hooks": {"Stop": {}}}"#, "Stop"),
    ("[]", "settings"),
];

#[test]
fn a_wrong_typed_key_is_an_error_naming_it() -> Test {
    let s = Scratch::new("shape")?;
    for (text, key) in SHAPES {
        s.with(text)?;
        for got in [
            install(&s.target(), Ide::ClaudeCode),
            uninstall(&s.target(), Ide::ClaudeCode),
        ] {
            let named =
                matches!(&got, Err(Error::Shape { key: k, .. }) if k == key);
            assert!(named, "{text}: {got:?}");
        }
    }
    Ok(())
}

#[test]
fn a_wrong_typed_claude_json_is_an_error_naming_the_key() -> Test {
    let s = Scratch::new("mcp-shape")?;
    for (text, key) in [
        (r#"{"mcpServers": 1}"#, "mcpServers"),
        ("[]", "claude.json"),
    ] {
        s.with_mcp(text)?;
        let got = install(&s.target(), Ide::ClaudeCode);
        let named =
            matches!(&got, Err(Error::Shape { key: k, .. }) if k == key);
        assert!(named, "{text}: {got:?}");
    }
    Ok(())
}

#[test]
fn uninstall_reads_mcp_servers_only_when_it_is_an_object() -> Test {
    let s = Scratch::new("mcp-shape-out")?;
    s.with_mcp("[]")?;
    let got = uninstall(&s.target(), Ide::ClaudeCode);
    assert!(matches!(&got, Err(Error::Shape { .. })), "{got:?}");
    s.with_mcp(r#"{"mcpServers": 1}"#)?;
    let back = uninstall(&s.target(), Ide::ClaudeCode)?;
    assert_eq!(back, both(Change::Unchanged));
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_private_file_and_its_backup_stay_private() -> Test {
    use std::os::unix::fs::PermissionsExt;
    let s = Scratch::new("private")?;
    let mcp = s.with_mcp(CLAUDE_JSON)?;
    fs::set_permissions(&mcp, fs::Permissions::from_mode(0o600))?;
    install(&s.target(), Ide::ClaudeCode)?;
    for file in [mcp.clone(), bak(&mcp)] {
        let mode = fs::metadata(&file)?.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "{file:?}");
    }
    Ok(())
}
