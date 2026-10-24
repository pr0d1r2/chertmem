use super::strip;
use crate::config::Grammar;
use crate::hook::Kind;
use crate::hook::tests::{Scratch, Test};

#[test]
fn text_without_a_tag_is_kept() {
    assert_eq!(strip("plain text, no secrets"), "plain text, no secrets");
    assert_eq!(strip(""), "");
}

#[test]
fn every_private_span_is_removed_with_its_tags() {
    let text = "a <private>key=1</private> b <private>x\ny</private>c";
    assert_eq!(strip(text), "a  b c");
}

#[test]
fn tags_match_in_any_case() {
    assert_eq!(strip("a<PRIVATE>s</Private>b"), "ab");
}

#[test]
fn an_unclosed_tag_drops_the_rest() {
    assert_eq!(strip("kept <private>secret to the end"), "kept ");
}

#[test]
fn spans_end_at_the_first_close() {
    let text = "<private>a<private>b</private>c</private>d";
    assert_eq!(strip(text), "c</private>d");
}

#[test]
fn text_around_a_span_keeps_its_bytes() {
    let text = "z\u{fc}rich <private>\u{e9}t\u{e9}</private> \u{2192} ok";
    assert_eq!(strip(text), "z\u{fc}rich  \u{2192} ok");
}

#[test]
fn a_lone_close_tag_is_kept() {
    assert_eq!(strip("a</private>b"), "a</private>b");
}

const PROMPT: &[u8] = br#"{"session_id":"s",
    "prompt":"deploy <private>token=hunter2</private>now"}"#;
const TOOL: &[u8] = br#"{"session_id":"s","tool_name":"Read",
    "tool_input":{"file_path":"a.rs"},
    "tool_response":{"content":"x <private>pw</private> y"},
    "tool_use_id":"t1"}"#;

#[test]
fn private_spans_never_reach_the_store() -> Test {
    let mut scratch = Scratch::new("private")?;
    scratch.config.compression.grammar = Grammar::Off;
    scratch.run(Kind::UserPromptSubmit, PROMPT);
    scratch.run(Kind::PostToolUse, TOOL);
    let found = scratch.observations()?;
    let bodies: Vec<_> = found.into_iter().map(|o| o.body).collect();
    assert_eq!(bodies.first().map(String::as_str), Some("deploy now"));
    assert_eq!(bodies.len(), 2);
    for body in &bodies {
        assert!(!body.contains("hunter2") && !body.contains("pw"), "{body}");
        assert!(!body.to_lowercase().contains("private"), "{body}");
    }
    Ok(())
}
