use super::parse;
use crate::hook::tests::{
    Res, SESSION, SESSION_2, SESSION_FAILURE, TWICE, Test, fixture, fixture_2,
};
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

/// `body` as the first recorded session's payload.
fn recorded(body: &str) -> Payload {
    Payload {
        session: SESSION.to_owned(),
        ..payload(body)
    }
}

#[test]
fn session_start_keeps_its_source() -> Test {
    let want = Payload {
        source: Some("startup".to_owned()),
        ..recorded("startup")
    };
    assert_eq!(parsed(Kind::SessionStart)?, want);
    Ok(())
}

#[test]
fn prompt_stop_and_end_keep_their_text() -> Test {
    let prompt = "Run exactly this shell command and reply with its output \
        only: echo chert-fixture";
    assert_eq!(parsed(Kind::UserPromptSubmit)?, recorded(prompt));
    assert_eq!(parsed(Kind::Stop)?, recorded("chert-fixture"));
    assert_eq!(parsed(Kind::SessionEnd)?, recorded("other"));
    Ok(())
}

#[test]
fn post_tool_use_keeps_tool_input_response_and_id() -> Test {
    let input = concat!(
        r#"{"command":"echo chert-fixture","#,
        r#""description":"Print fixture string"}"#,
    );
    let response = concat!(
        r#"{"interrupted":false,"isImage":false,"#,
        r#""noOutputExpected":false,"stderr":"","stdout":"chert-fixture"}"#,
    );
    let want = Payload {
        identity: Some("toolu_01FIXTURE000000000000001".to_owned()),
        ..recorded(&format!("Bash\n{input}\n{response}"))
    };
    assert_eq!(parsed(Kind::PostToolUse)?, want);
    Ok(())
}

/// The recorded failure has no `tool_response`: its `error` takes the
/// third line (src/hook T9).
#[test]
fn post_tool_use_failure_keeps_tool_input_error_and_id() -> Test {
    let input = r#"{"command":"false","description":"Run the false command"}"#;
    let want = Payload {
        session: SESSION_FAILURE.to_owned(),
        identity: Some("toolu_01FIXTURE000000000000003".to_owned()),
        ..payload(&format!("Bash\n{input}\nExit code 1"))
    };
    assert_eq!(parsed(Kind::PostToolUseFailure)?, want);
    Ok(())
}

#[test]
fn a_failure_without_its_fields_gives_empty_lines() {
    let bare = br#"{"session_id":"abc123"}"#;
    let got = parse(Kind::PostToolUseFailure, bare);
    assert_eq!(got, Some(payload("\n\n")));
}

/// The second recorded session ran the same prompt: same payloads but
/// its session and, for the tool call, its `tool_use_id`.
#[test]
fn the_second_session_differs_only_in_session_and_tool_use_id() -> Test {
    for kind in TWICE {
        let raw = fixture_2(kind)?;
        let second = parse(kind, &raw).ok_or("fixture did not parse")?;
        assert_eq!(second.session, SESSION_2, "{kind:?}");
        let first = parsed(kind)?;
        let same = |p: Payload| (p.body, p.paths, p.source);
        assert_eq!(same(second.clone()), same(first.clone()), "{kind:?}");
        let ids = (first.identity.is_some(), second.identity != first.identity);
        let tool = kind == Kind::PostToolUse;
        assert_eq!(ids, (tool, tool), "{kind:?}");
    }
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

/// A failed call is a tool call too: same identity and paths (T9).
#[test]
fn a_tool_failure_has_the_identity_and_paths_of_a_tool_use() {
    let text = br#"{"session_id":"abc123","tool_use_id":"toolu_1",
        "tool_input":{"file_path":"a.rs"}}"#;
    let tool = parse(Kind::PostToolUse, text).map(|p| (p.identity, p.paths));
    let failed = parse(Kind::PostToolUseFailure, text);
    assert_eq!(failed.map(|p| (p.identity, p.paths)), tool);
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
