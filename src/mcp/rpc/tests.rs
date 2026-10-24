use std::error::Error as StdError;

use serde_json::{Value, json};

use super::{Frame, INVALID, LATEST, PARSE, fail, initialize, ok, parse};

type Test = Result<(), Box<dyn StdError>>;

fn bad(line: &str) -> Option<Value> {
    match parse(line) {
        Frame::Bad(v) => Some(v),
        Frame::Request { .. } | Frame::Silent => None,
    }
}

/// The id, method and params of a request line.
fn request(line: &str) -> Result<(Value, String, Value), Box<dyn StdError>> {
    match parse(line) {
        Frame::Request { id, method, params } => Ok((id, method, params)),
        Frame::Bad(_) | Frame::Silent => Err(line.into()),
    }
}

#[test]
fn a_request_carries_its_id_method_and_params() -> Test {
    let line = r#"{"jsonrpc":"2.0","id":7,"method":"ping","params":{"a":1}}"#;
    let (id, method, params) = request(line)?;
    assert_eq!((id, method.as_str()), (json!(7), "ping"));
    assert_eq!(params, json!({"a": 1}));
    Ok(())
}

#[test]
fn a_request_without_params_has_null_params() -> Test {
    let line = r#"{"jsonrpc":"2.0","id":"x","method":"tools/list"}"#;
    let (id, _, params) = request(line)?;
    assert_eq!((id, params), (json!("x"), Value::Null));
    Ok(())
}

#[test]
fn notifications_and_client_responses_are_never_answered() {
    for line in [
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":3,"result":{}}"#,
        r#"{"jsonrpc":"2.0","id":4,"error":{"code":1,"message":"m"}}"#,
    ] {
        assert!(matches!(parse(line), Frame::Silent), "{line}");
    }
}

#[test]
fn unparsable_json_is_a_parse_error_with_a_null_id() {
    assert_eq!(bad("{nope"), Some(fail(Value::Null, PARSE, "Parse error")));
}

#[test]
fn a_frame_that_is_not_a_request_is_invalid() {
    let invalid = |id| Some(fail(id, INVALID, "Invalid Request"));
    assert_eq!(bad("[1,2]"), invalid(Value::Null));
    assert_eq!(bad(r#"{"jsonrpc":"2.0","id":5}"#), invalid(json!(5)));
    let old = r#"{"jsonrpc":"1.0","id":6,"method":"ping"}"#;
    assert_eq!(bad(old), invalid(json!(6)));
    let odd = r#"{"jsonrpc":"2.0","id":[1],"method":"ping"}"#;
    assert_eq!(bad(odd), invalid(Value::Null));
}

#[test]
fn ok_and_fail_frame_a_result_and_an_error() {
    let want = json!({"jsonrpc": "2.0", "id": 1, "result": {"x": true}});
    assert_eq!(ok(json!(1), json!({"x": true})), want);
    let e = json!({"jsonrpc": "2.0", "id": 2,
        "error": {"code": -32601, "message": "m"}});
    assert_eq!(fail(json!(2), -32601, "m"), e);
}

#[test]
fn initialize_echoes_a_supported_version_else_the_latest() {
    let asked = |v: &str| initialize(&json!({"protocolVersion": v}));
    let version = |r: &Value| r.get("protocolVersion").cloned();
    assert_eq!(version(&asked("2024-11-05")), Some(json!("2024-11-05")));
    assert_eq!(version(&asked("2025-06-18")), Some(json!("2025-06-18")));
    assert_eq!(version(&asked("1999-01-01")), Some(json!(LATEST)));
    assert_eq!(version(&initialize(&Value::Null)), Some(json!(LATEST)));
}

#[test]
fn initialize_names_the_server_and_offers_tools() {
    let r = initialize(&json!({"protocolVersion": LATEST}));
    let info = json!({"name": "chert", "version": env!("CARGO_PKG_VERSION")});
    assert_eq!(r.get("serverInfo"), Some(&info));
    assert_eq!(r.get("capabilities"), Some(&json!({"tools": {}})));
}
