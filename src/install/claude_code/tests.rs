//! chert's entries in Claude Code's settings: commands, ownership,
//! wiring and unwiring JSON values.

use serde_json::{Value, json};

use super::{command, owned, quote, server, unquote, unwire, wire, wiring};
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
fn the_command_runs_the_event_with_the_ide() {
    let got = command("chert", Kind::UserPromptSubmit);
    assert_eq!(got, "chert hook run user-prompt-submit --ide claude-code");
}

#[test]
fn owned_knows_any_chert_for_its_own_event_only() {
    let stop = Kind::Stop;
    assert!(owned(&command("chert", stop), stop));
    assert!(owned(&command("/a b/chert", stop), stop));
    assert!(!owned(&command("chert", stop), Kind::SessionEnd));
    assert!(!owned(&command("/bin/other", stop), stop));
    assert!(!owned("lint.sh", stop));
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
fn unwire_keeps_containers_it_did_not_empty() -> Test {
    let before = json!({"hooks": {}, "mcpServers": {}});
    let mut value = before.clone();
    unwire(&mut value)?;
    assert_eq!(value, before);
    Ok(())
}

#[test]
fn unwire_keeps_a_user_hook_sharing_chert_s_group() -> Test {
    let mine = json!({"type": "command", "command": "lint.sh"});
    let chert =
        json!({"type": "command", "command": command(CHERT, Kind::Stop)});
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

fn states(value: &Value) -> Vec<WireState> {
    wiring(value, CHERT).into_iter().map(|w| w.state).collect()
}

/// Every entry's name, in the order `wiring` gives them.
const NAMES: [&str; 6] = [
    "hook SessionStart",
    "hook UserPromptSubmit",
    "hook PostToolUse",
    "hook Stop",
    "hook SessionEnd",
    "mcp server chert",
];

#[test]
fn wiring_names_each_entry() {
    let got = wiring(&json!({}), CHERT);
    let names: Vec<&str> = got.iter().map(|w| w.name.as_str()).collect();
    assert_eq!(names, NAMES);
}

#[test]
fn wiring_is_missing_until_wired() -> Test {
    assert_eq!(states(&json!({})), vec![WireState::Missing; 6]);
    assert_eq!(states(&json!([])), vec![WireState::Missing; 6]);
    let mut value = json!({});
    wire(&mut value, CHERT)?;
    assert_eq!(states(&value), vec![WireState::Ok; 6]);
    Ok(())
}

#[test]
fn wiring_tells_another_chert_s_hook_from_this_one() -> Test {
    let mut value = json!({});
    wire(&mut value, "/old/chert")?;
    let got = wiring(&value, CHERT);
    let old = command("/old/chert", Kind::SessionStart);
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
    wire(&mut value, "/old/chert")?;
    let got = wiring(&value, CHERT);
    let found = server("/old/chert").to_string();
    let last = got.last().map(|w| w.state.clone());
    assert_eq!(last, Some(WireState::Other(found)));
    Ok(())
}
