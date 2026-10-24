use super::parse;
use crate::hook::tests::{Res, Test, fixture};
use crate::hook::{Kind, Payload};

fn parsed(kind: Kind) -> Res<Payload> {
    Ok(parse(kind, &fixture(kind)?).ok_or("fixture did not parse")?)
}

fn payload(body: &str) -> Payload {
    Payload {
        session: "abc123".to_owned(),
        identity: None,
        body: body.to_owned(),
        paths: Vec::new(),
        source: None,
    }
}

#[test]
fn session_start_keeps_its_source() -> Test {
    let want = Payload {
        source: Some("startup".to_owned()),
        ..payload("startup")
    };
    assert_eq!(parsed(Kind::SessionStart)?, want);
    Ok(())
}

#[test]
fn prompt_stop_and_end_keep_their_text() -> Test {
    let prompt = "Write a function to calculate the factorial of a number";
    assert_eq!(parsed(Kind::UserPromptSubmit)?, payload(prompt));
    let said = "I have added the factorial function in src/factorial.rs \
        and the tests pass.";
    assert_eq!(parsed(Kind::Stop)?, payload(said));
    assert_eq!(parsed(Kind::SessionEnd)?, payload("other"));
    Ok(())
}

#[test]
fn post_tool_use_keeps_tool_input_response_id_and_path() -> Test {
    let path = "/workspace/project/src/factorial.rs";
    let input = format!(
        "{{\"content\":\"pub fn factorial(n: u64) -> u64 {{ \
         (1..=n).product() }}\\n\",\"file_path\":\"{path}\"}}"
    );
    let response = format!("{{\"filePath\":\"{path}\",\"type\":\"create\"}}");
    let want = Payload {
        identity: Some("toolu_01ABC123".to_owned()),
        paths: vec![path.to_owned()],
        ..payload(&format!("Write\n{input}\n{response}"))
    };
    assert_eq!(parsed(Kind::PostToolUse)?, want);
    Ok(())
}

#[test]
fn unknown_fields_are_ignored() {
    let plain = br#"{"session_id":"abc123","prompt":"hi"}"#;
    let extra = br#"{"session_id":"abc123","prompt":"hi","new":{"x":1}}"#;
    let kind = Kind::UserPromptSubmit;
    assert_eq!(parse(kind, plain), Some(payload("hi")));
    assert_eq!(parse(kind, extra), parse(kind, plain));
}

#[test]
fn missing_event_fields_give_an_empty_body() {
    let bare = br#"{"session_id":"abc123"}"#;
    for kind in [Kind::UserPromptSubmit, Kind::Stop, Kind::SessionEnd] {
        assert_eq!(parse(kind, bare), Some(payload("")), "{kind:?}");
    }
    assert_eq!(parse(Kind::PostToolUse, bare), Some(payload("\n\n")));
}

#[test]
fn only_post_tool_use_has_an_identity_or_paths() {
    let text = br#"{"session_id":"abc123","tool_use_id":"toolu_1",
        "tool_input":{"file_path":"a.rs"}}"#;
    assert_eq!(parse(Kind::UserPromptSubmit, text), Some(payload("")));
    let empty = br#"{"session_id":"abc123","tool_use_id":""}"#;
    let got = parse(Kind::PostToolUse, empty).and_then(|p| p.identity);
    assert_eq!(got, None);
}

#[test]
fn every_tool_path_key_is_read() {
    let text = br#"{"session_id":"s","tool_input":{"path":"d",
        "notebook_path":"n.ipynb","file_path":"f","pattern":"*.rs"}}"#;
    let paths = parse(Kind::PostToolUse, text).map(|p| p.paths);
    let want = ["f", "d", "n.ipynb"].map(str::to_owned).to_vec();
    assert_eq!(paths, Some(want));
}

#[test]
fn unreadable_payloads_give_none() {
    let bad: [&[u8]; 6] = [
        b"",
        b"not json",
        b"[1,2]",
        br#"{"session_id":7}"#,
        br#"{"session_id":""}"#,
        b"{\"session_id\":\"\xff\"}",
    ];
    for text in bad {
        assert_eq!(parse(Kind::Stop, text), None, "{text:?}");
    }
}
