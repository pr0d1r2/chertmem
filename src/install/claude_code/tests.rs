//! chert's entries in Claude Code's settings and `~/.claude.json`:
//! commands, ownership, wiring and unwiring JSON values.

use serde_json::{Value, json};

use super::{EVENTS, command, owned, quote, server, unquote};
use super::{serve, unserve, unwire, wire, wiring};
use crate::hook::Kind;
use crate::install::tests::{CHERT, Test};
use crate::install::{Wire, WireState};

#[test]
fn quote_leaves_plain_words_and_single_quotes_the_rest() {
    assert_eq!(quote("/usr/bin/chert"), "/usr/bin/chert");
    assert_eq!(quote("a b"), "'a b'");
    assert_eq!(quote("it's"), r"'it'\''s'");
    assert_eq!(quote(""), "''");
    for word in ["/usr/bin/chert", "a b", "it's", "", "$x"] {
        assert_eq!(unquote(&quote(word)), word);
    }
}

#[test]
fn every_hook_event_chert_reads_is_registered() {
    for kind in Kind::ALL {
        let named = EVENTS.iter().any(|&(e, _)| e == kind.as_str());
        assert!(named, "{kind:?}");
    }
    let failure = ("post-tool-use-failure", "PostToolUseFailure");
    assert!(EVENTS.contains(&failure));
}

#[test]
fn the_command_runs_the_event_with_the_ide() {
    let got = command("chert", "user-prompt-submit");
    assert_eq!(got, "chert hook run user-prompt-submit --ide claude-code");
}

#[test]
fn owned_knows_any_chert_for_its_own_event_only() {
    let stop = "stop";
    assert!(owned(&command("chert", stop), stop));
    assert!(owned(&command("/a b/chert", stop), stop));
    assert!(!owned(&command("chert", stop), "session-end"));
    assert!(!owned(&command("/bin/other", stop), stop));
    assert!(!owned("lint.sh", stop));
}

#[test]
fn the_server_is_what_claude_mcp_add_writes_for_stdio() {
    let want = json!({
        "type": "stdio", "command": CHERT, "args": ["mcp"], "env": {},
    });
    assert_eq!(server(CHERT), want);
}

#[test]
fn wire_then_unwire_is_the_value_before() -> Test {
    let before = json!({
        "hooks": {"Stop": [{"hooks": [{"type": "command", "command": "x"}]}]},
        "mcpServers": {"other": {}},
        "model": "opus",
    });
    let mut value = before.clone();
    wire(&mut value, CHERT)?;
    assert_ne!(value, before);
    unwire(&mut value)?;
    assert_eq!(value, before);
    Ok(())
}

#[test]
fn wire_writes_hooks_only_and_drops_a_server_chert_put_there() -> Test {
    let mut value = json!({"mcpServers": {"chert": {"command": "chert"}}});
    wire(&mut value, CHERT)?;
    assert_eq!(value.get("mcpServers"), None);
    assert!(value.get("hooks").is_some_and(Value::is_object));
    Ok(())
}

#[test]
fn serve_then_unserve_is_the_value_before() -> Test {
    let before = json!({"mcpServers": {"other": {}}, "numStartups": 3});
    let mut value = before.clone();
    serve(&mut value, CHERT)?;
    let got = value.pointer("/mcpServers/chert").cloned();
    assert_eq!(got, Some(server(CHERT)));
    unserve(&mut value)?;
    assert_eq!(value, before);
    let mut value = json!({});
    serve(&mut value, CHERT)?;
    unserve(&mut value)?;
    assert_eq!(value, json!({}));
    Ok(())
}

#[test]
fn unwire_keeps_containers_it_did_not_empty() -> Test {
    let before = json!({"hooks": {}, "mcpServers": {}});
    let mut value = before.clone();
    unwire(&mut value)?;
    assert_eq!(value, before);
    unserve(&mut value)?;
    assert_eq!(value, before);
    Ok(())
}

#[test]
fn unwire_keeps_a_user_hook_sharing_chert_s_group() -> Test {
    let mine = json!({"type": "command", "command": "lint.sh"});
    let chert = json!({"type": "command", "command": command(CHERT, "stop")});
    let mut value = json!({"hooks": {"Stop": [{"hooks": [mine, chert]}]}});
    unwire(&mut value)?;
    assert_eq!(value, json!({"hooks": {"Stop": [{"hooks": [mine]}]}}));
    Ok(())
}

#[test]
fn a_group_without_a_hook_list_is_kept() -> Test {
    let before = json!({"hooks": {"Stop": [{"matcher": "x"}]}});
    let mut value = before.clone();
    unwire(&mut value)?;
    assert_eq!(value, before);
    Ok(())
}

/// Each entry's state with `settings` and `mcp` as the files' JSON.
fn states(settings: &Value, mcp: &Value) -> Vec<WireState> {
    let got = wiring(settings, mcp, CHERT);
    got.into_iter().map(|w| w.state).collect()
}

/// Every entry's name, in the order `wiring` gives them.
const NAMES: [&str; 7] = [
    "hook SessionStart",
    "hook UserPromptSubmit",
    "hook PostToolUse",
    "hook PostToolUseFailure",
    "hook Stop",
    "hook SessionEnd",
    "mcp server chert",
];

#[test]
fn wiring_names_each_entry() {
    let got = wiring(&json!({}), &json!({}), CHERT);
    let names: Vec<&str> = got.iter().map(|w| w.name.as_str()).collect();
    assert_eq!(names, NAMES);
}

#[test]
fn wiring_is_missing_until_wired() -> Test {
    let none = vec![WireState::Missing; 7];
    assert_eq!(states(&json!({}), &json!({})), none);
    assert_eq!(states(&json!([]), &json!([])), none);
    let (mut settings, mut mcp) = (json!({}), json!({}));
    wire(&mut settings, CHERT)?;
    serve(&mut mcp, CHERT)?;
    assert_eq!(states(&settings, &mcp), vec![WireState::Ok; 7]);
    assert_eq!(states(&settings, &settings).last(), none.last());
    Ok(())
}

#[test]
fn wiring_tells_another_chert_s_hook_from_this_one() -> Test {
    let mut value = json!({});
    wire(&mut value, "/old/chert")?;
    let got = wiring(&value, &json!({}), CHERT);
    let old = command("/old/chert", "session-start");
    let want = Wire {
        name: "hook SessionStart".to_owned(),
        state: WireState::Other(old),
    };
    assert_eq!(got.first(), Some(&want));
    Ok(())
}

#[test]
fn wiring_tells_another_chert_s_server_from_this_one() -> Test {
    let mut value = json!({});
    serve(&mut value, "/old/chert")?;
    let got = wiring(&json!({}), &value, CHERT);
    let found = server("/old/chert").to_string();
    let last = got.last().map(|w| w.state.clone());
    assert_eq!(last, Some(WireState::Other(found)));
    Ok(())
}
