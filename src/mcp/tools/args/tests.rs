use std::error::Error as StdError;

use serde_json::{Value, json};

use super::Args;
use crate::store::Ref;

type Test = Result<(), Box<dyn StdError>>;

#[test]
fn text_is_required_and_non_empty() {
    let v = json!({"query": "auth", "empty": "", "n": 3});
    let a = Args::new("search", &v);
    assert_eq!(a.text("query"), Ok("auth".to_owned()));
    for key in ["empty", "n", "missing"] {
        let e = a.text(key).err().unwrap_or_default();
        let want = format!("Invalid arguments for tool search: {key} must");
        assert!(e.starts_with(&want), "{e}");
    }
}

#[test]
fn count_is_optional_and_bounded() {
    let v = json!({"ok": 50, "zero": 0, "big": 51, "neg": -1,
        "float": 1.5, "text": "3", "nul": null});
    let a = Args::new("search", &v);
    assert_eq!(a.count("ok", 50), Ok(Some(50)));
    assert_eq!(a.count("missing", 50), Ok(None));
    assert_eq!(a.count("nul", 50), Ok(None));
    for key in ["zero", "big", "neg", "float", "text"] {
        assert!(a.count(key, 50).is_err(), "{key}");
    }
}

#[test]
fn absent_arguments_read_as_an_empty_object() {
    let a = Args::new("timeline", &Value::Null);
    assert_eq!(a.count("limit", 10), Ok(None));
    assert_eq!(a.flag("expand"), Ok(None));
    assert!(a.text("session_id").is_err());
}

#[test]
fn flag_is_an_optional_boolean() {
    let v = json!({"yes": true, "no": false, "odd": "true"});
    let a = Args::new("get_observations", &v);
    assert_eq!(a.flag("yes"), Ok(Some(true)));
    assert_eq!(a.flag("no"), Ok(Some(false)));
    assert!(a.flag("odd").is_err());
}

#[test]
fn refs_parse_chert_refs_within_bounds() -> Test {
    let one = Ref::parse("chert:01KABCDE")?;
    let v = json!({"ids": ["chert:01KABCDE"], "none": [],
        "bad": ["chert:1"], "int": [3], "many": vec!["chert:01KABCDE"; 3]});
    let a = Args::new("get_observations", &v);
    assert_eq!(a.refs("ids", 2), Ok(vec![one.clone()]));
    for key in ["none", "bad", "int", "many", "missing"] {
        assert!(a.refs(key, 2).is_err(), "{key}");
    }
    assert_eq!(a.reference("missing"), Ok(None));
    let around = json!({"around_id": "chert:01KABCDE", "bad": 7});
    let a = Args::new("timeline", &around);
    assert_eq!(a.reference("around_id"), Ok(Some(one)));
    assert!(a.reference("bad").is_err());
    Ok(())
}
