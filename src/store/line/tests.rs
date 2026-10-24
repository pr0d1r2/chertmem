use super::{decode, encode};
use crate::store::session::{Begun, Ended};
use crate::store::tests::{Test, event};
use crate::store::write::{Item, Note, Row};
use crate::store::{Scope, SessionStart, Summary};

const ID: &str = "01JB2X3Y4Z5A6B7C8D9E0F1G2H";

fn observation(identity: Option<&str>, body: &str) -> Item {
    Item::Observation(Row {
        id: ID.into(),
        project: "proj-0123".into(),
        event: event(1_709_211_600_000, identity, body),
    })
}

fn summary(scope: Scope, body: &str) -> Item {
    Item::Summary(Note {
        id: ID.into(),
        project: "proj-0123".into(),
        summary: Summary {
            ts: 1_709_211_600_000,
            session: "session-1".into(),
            scope,
            ide: "claude-code".into(),
            body: body.into(),
        },
    })
}

fn begun() -> Item {
    Item::Begun(Begun {
        project: "proj-0123".into(),
        start: SessionStart {
            id: "session-1".into(),
            ide: "claude-code".into(),
            cwd: "/work/a \"b\"".into(),
            ts: 1_709_211_600_000,
        },
    })
}

fn ended() -> Item {
    Item::Ended(Ended {
        project: "proj-0123".into(),
        id: "session-1".into(),
        ts: 1_709_211_600_001,
    })
}

#[test]
fn a_line_round_trips_every_field() -> Test {
    let items = [
        observation(Some("toolu_1"), "a \"quoted\"\nline \u{e9}"),
        observation(None, ""),
        summary(Scope::Turn, "did x\tthen y"),
        summary(Scope::Session, ""),
        begun(),
        ended(),
    ];
    for item in items {
        let line = encode(&item)?;
        assert!(!line.contains('\n'), "{line}");
        assert_eq!(decode(&line), Some(item));
    }
    Ok(())
}

#[test]
fn anything_else_decodes_to_none() -> Test {
    for line in bad()?.into_iter().chain(bad_sessions()?) {
        assert_eq!(decode(&line), None, "{line}");
    }
    Ok(())
}

/// Session lines a field away from good ones.
fn bad_sessions() -> Result<[String; 3], Box<dyn std::error::Error>> {
    let (start, end) = (encode(&begun())?, encode(&ended())?);
    Ok([
        start.replace("\"cwd\"", "\"dir\""),
        start.replace("\"1709211600000\"", "\"soon\""),
        end.replace("\"1709211600001\"", "\"soon\""),
    ])
}

/// Lines that are not spool lines, most a field away from one.
fn bad() -> Result<[String; 8], Box<dyn std::error::Error>> {
    let good = encode(&observation(None, "b"))?;
    let note = encode(&summary(Scope::Turn, "b"))?;
    Ok([
        "not json".to_owned(),
        "[1, 2]".to_owned(),
        good.replace("\"observation\"", "\"summary-x\""),
        good.replace("\"session\"", "\"sess\""),
        good.replace("\"1709211600000\"", "\"soon\""),
        good.replace("\"identity\":null", "\"identity\":7"),
        note.replace("\"turn\"", "\"week\""),
        note.replace("\"scope\"", "\"scopes\""),
    ])
}
