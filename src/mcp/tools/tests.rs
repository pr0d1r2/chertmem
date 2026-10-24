use std::error::Error as StdError;

use serde_json::{Value, json};

use super::{Call, list, parse};
use crate::store::Ref;

type Test = Result<(), Box<dyn StdError>>;

/// Each listed tool's name with its property names and required list.
fn listed() -> Vec<(String, Vec<String>, Value)> {
    let tools = list().get("tools").cloned().unwrap_or_default();
    let tools = tools.as_array().cloned().unwrap_or_default();
    tools.iter().map(outline).collect()
}

fn outline(t: &Value) -> (String, Vec<String>, Value) {
    let name = t.get("name").and_then(Value::as_str);
    let schema = t.get("inputSchema").cloned().unwrap_or_default();
    let props = schema.get("properties").and_then(Value::as_object);
    let keys = props.map(|p| p.keys().cloned().collect());
    let required = schema.get("required").cloned().unwrap_or_default();
    let name = name.unwrap_or_default().to_owned();
    (name, keys.unwrap_or_default(), required)
}

#[test]
fn tools_list_golden_names_and_params_as_cavemem() {
    let want = [
        ("search", vec!["limit", "query"], json!(["query"])),
        (
            "timeline",
            vec!["around_id", "limit", "session_id"],
            json!(["session_id"]),
        ),
        ("get_observations", vec!["expand", "ids"], json!(["ids"])),
    ];
    let want: Vec<(String, Vec<String>, Value)> = want
        .into_iter()
        .map(|(n, k, r)| (n.into(), k.into_iter().map(Into::into).collect(), r))
        .collect();
    assert_eq!(listed(), want);
}

#[test]
fn every_tool_has_a_description_and_an_object_schema() {
    let tools = list().get("tools").cloned().unwrap_or_default();
    for t in tools.as_array().cloned().unwrap_or_default() {
        let text = t.get("description").and_then(Value::as_str);
        assert!(text.is_some_and(|d| !d.is_empty()), "{t}");
        let kind = t.pointer("/inputSchema/type");
        assert_eq!(kind, Some(&json!("object")), "{t}");
    }
}

fn call(name: &str, arguments: &Value) -> Result<Call, String> {
    parse(&json!({"name": name, "arguments": arguments}))
}

#[test]
fn search_takes_query_and_limit() {
    let got = call("search", &json!({"query": "auth", "limit": 5}));
    let want = Call::Search {
        query: "auth".into(),
        limit: Some(5),
    };
    assert_eq!(got, Ok(want));
    assert!(call("search", &json!({"query": "a", "limit": 51})).is_err());
    assert!(call("search", &json!({})).is_err());
}

#[test]
fn timeline_takes_session_around_and_limit() -> Test {
    let at = Ref::parse("chert:01KABCDE")?;
    let args = json!({"session_id": "s1", "around_id": "chert:01KABCDE",
        "limit": 200});
    let want = Call::Timeline {
        session_id: "s1".into(),
        around_id: Some(at),
        limit: Some(200),
    };
    assert_eq!(call("timeline", &args), Ok(want));
    let over = json!({"session_id": "s1", "limit": 201});
    assert!(call("timeline", &over).is_err());
    Ok(())
}

#[test]
fn get_observations_expand_defaults_false() -> Test {
    let one = Ref::parse("chert:01KABCDE")?;
    let got = call("get_observations", &json!({"ids": ["chert:01KABCDE"]}));
    let want = Call::Get {
        ids: vec![one.clone()],
        expand: false,
    };
    assert_eq!(got, Ok(want));
    let asked = json!({"ids": ["chert:01KABCDE"], "expand": true});
    let want = Call::Get {
        ids: vec![one],
        expand: true,
    };
    assert_eq!(call("get_observations", &asked), Ok(want));
    Ok(())
}

#[test]
fn an_unknown_or_unnamed_tool_is_refused() {
    let unknown = call("list_everything", &json!({}));
    assert_eq!(unknown, Err("Tool list_everything not found".to_owned()));
    assert!(parse(&json!({"arguments": {}})).is_err());
    assert!(parse(&Value::Null).is_err());
}
