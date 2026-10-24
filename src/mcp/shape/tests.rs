//! Golden JSON per tool (V1): cavemem 0.2.1's field names, chert refs
//! as ids.

use std::error::Error as StdError;

use serde_json::json;

use super::{content, entries, error, hits, observations};
use crate::grammar;
use crate::search::{Entry, Hit};
use crate::store::{Lookup, Observation, Ref};

type Test = Result<(), Box<dyn StdError>>;

fn observation(r: &Ref, body: &str) -> Observation {
    Observation {
        reference: r.clone(),
        id: format!("{}000000000000000000", r.as_str()),
        project: "p-0000000000000000".into(),
        ts: 1_767_225_600_000,
        session: "s1".into(),
        ide: "claude-code".into(),
        kind: "post-tool-use".into(),
        identity: None,
        body: body.into(),
    }
}

#[test]
fn search_hits_golden() -> Test {
    let hit = Hit {
        reference: Ref::parse("chert:01KABCDE")?,
        score: 1.5,
        snippet: "[auth] mw fix".into(),
        session: "s1".into(),
        ts: 1_767_225_600_000,
    };
    let want = json!([{"id": "chert:01KABCDE", "score": 1.5,
        "snippet": "[auth] mw fix", "session_id": "s1",
        "ts": 1_767_225_600_000_u64}]);
    assert_eq!(hits(&[hit]), want);
    assert_eq!(hits(&[]), json!([]));
    Ok(())
}

#[test]
fn timeline_entries_golden() -> Test {
    let entry = Entry {
        reference: Ref::parse("chert:01KABCDF")?,
        kind: "user-prompt-submit".into(),
        ts: 7,
    };
    let want = json!([{"id": "chert:01KABCDF",
        "kind": "user-prompt-submit", "ts": 7}]);
    assert_eq!(entries(&[entry]), want);
    Ok(())
}

#[test]
fn get_observations_golden_compressed_by_default() -> Test {
    let (a, gone) =
        (Ref::parse("chert:01KABCDG")?, Ref::parse("chert:01KZZZZZ")?);
    let body = "fix auth mw";
    let found = [Lookup::Found(observation(&a, body)), Lookup::Dropped(gone)];
    let want = json!([
        {"id": "chert:01KABCDG", "session_id": "s1",
         "kind": "post-tool-use", "ts": 1_767_225_600_000_u64,
         "content": body, "metadata": null},
        {"id": "chert:01KZZZZZ", "dropped": true},
    ]);
    assert_eq!(observations(&found, false), want);
    Ok(())
}

#[test]
fn get_observations_expands_on_request() -> Test {
    let a = Ref::parse("chert:01KABCDH")?;
    let body = "fix auth mw";
    let got = observations(&[Lookup::Found(observation(&a, body))], true);
    let text = got.pointer("/0/content");
    assert_eq!(text, Some(&json!(grammar::expand(body))));
    assert_ne!(grammar::expand(body), body);
    Ok(())
}

#[test]
fn a_result_is_1_text_item_and_an_error_sets_is_error() {
    let value = json!([{"id": "chert:01KABCDE"}]);
    let want = json!({"content": [{"type": "text",
        "text": "[{\"id\":\"chert:01KABCDE\"}]"}]});
    assert_eq!(content(&value), want);
    let e = json!({"content": [{"type": "text", "text": "no"}],
        "isError": true});
    assert_eq!(error("no"), e);
}
