use crate::hook::Kind;
use crate::hook::tests::{NOW, Res, SESSION, Scratch, Test, fixture};
use crate::store::Session;

/// The `cwd` every recorded fixture reports (src/hook R1).
const CWD: &str = "/workspace/project";

fn sessions(scratch: &Scratch) -> Res<Vec<Session>> {
    Ok(scratch.store()?.sessions(10)?)
}

fn row(ended_at: Option<u64>) -> Session {
    Session {
        id: SESSION.to_owned(),
        ide: "claude-code".to_owned(),
        cwd: CWD.to_owned(),
        started_at: NOW,
        ended_at,
    }
}

#[test]
fn session_start_records_cwd_and_start() -> Test {
    let scratch = Scratch::new("session-start-row")?;
    let kind = Kind::SessionStart;
    let out = scratch.run(kind, &fixture(kind)?);
    assert_eq!((out.exit, out.note), (0, None));
    assert_eq!(sessions(&scratch)?, [row(None)]);
    Ok(())
}

#[test]
fn session_end_records_the_end() -> Test {
    let scratch = Scratch::new("session-end-row")?;
    let start = Kind::SessionStart;
    scratch.run(start, &fixture(start)?);
    let end = Kind::SessionEnd;
    let out = scratch.run_at(end, &fixture(end)?, NOW + 9);
    assert_eq!((out.exit, out.note), (0, None));
    assert_eq!(sessions(&scratch)?, [row(Some(NOW + 9))]);
    Ok(())
}

#[test]
fn a_start_without_cwd_records_an_empty_one() -> Test {
    let scratch = Scratch::new("session-no-cwd")?;
    scratch.run(Kind::SessionStart, br#"{"session_id":"s1"}"#);
    let found = sessions(&scratch)?;
    let got: Vec<_> = found.iter().map(|s| (s.id.as_str(), &*s.cwd)).collect();
    assert_eq!(got, [("s1", "")]);
    Ok(())
}

#[test]
fn other_events_record_no_session() -> Test {
    let scratch = Scratch::new("session-none")?;
    for kind in [Kind::UserPromptSubmit, Kind::PostToolUse, Kind::Stop] {
        scratch.run(kind, &fixture(kind)?);
    }
    assert_eq!(sessions(&scratch)?, []);
    Ok(())
}
