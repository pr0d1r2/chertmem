use super::{TURNS, rollup};
use crate::hook::Kind;
use crate::hook::tests::{NOW, Res, SESSION, Scratch, Test, fixture};
use crate::store::{Scope, Summary};

fn stop(session: &str, said: &str) -> Vec<u8> {
    let text = serde_json::json!({
        "session_id": session,
        "hook_event_name": "Stop",
        "stop_hook_active": false,
        "last_assistant_message": said,
    });
    text.to_string().into_bytes()
}

fn end(session: &str) -> Vec<u8> {
    let text = serde_json::json!({"session_id": session, "reason": "other"});
    text.to_string().into_bytes()
}

fn summaries(scratch: &Scratch, scope: Scope) -> Res<Vec<Summary>> {
    Ok(scratch.store()?.summaries(None, scope)?)
}

/// `count` stops in `session`, 1 ms apart from `NOW`, then its end.
fn session_of(scratch: &Scratch, session: &str, count: u64) {
    for n in 0..count {
        let said = format!("Turn {n}: I fixed the build in src/lib.rs.");
        scratch.run_at(
            Kind::Stop,
            &stop(session, &said),
            NOW.saturating_add(n),
        );
    }
    scratch.run_at(Kind::SessionEnd, &end(session), NOW.saturating_add(count));
}

#[test]
fn stop_writes_its_message_as_a_turn_summary() -> Test {
    let scratch = Scratch::new("turn")?;
    let out = scratch.run(Kind::Stop, &fixture(Kind::Stop)?);
    assert_eq!(out.note, None);
    let turns = summaries(&scratch, Scope::Turn)?;
    let observed = scratch.observations()?;
    let body = observed.first().map(|o| o.body.clone());
    assert_eq!(turns.len(), 1);
    let turn = turns.first().ok_or("no turn")?;
    assert_eq!(Some(&turn.body), body.as_ref());
    assert_eq!(
        (turn.session.as_str(), turn.ide.as_str()),
        (SESSION, "claude-code")
    );
    assert_eq!(turn.ts, NOW);
    Ok(())
}

#[test]
fn a_stop_without_a_message_writes_no_turn_summary() -> Test {
    let scratch = Scratch::new("emptyturn")?;
    scratch.run(Kind::Stop, br#"{"session_id":"s"}"#);
    assert!(summaries(&scratch, Scope::Turn)?.is_empty());
    assert_eq!(scratch.observations()?.len(), 1);
    Ok(())
}

#[test]
fn session_end_rolls_up_the_first_20_turns() -> Test {
    let scratch = Scratch::new("rollup")?;
    session_of(&scratch, "other", 2);
    session_of(&scratch, "s", 25);
    let turns = scratch.store()?.summaries(Some("s"), Scope::Turn)?;
    let sessions = scratch.store()?.summaries(Some("s"), Scope::Session)?;
    let want: Vec<_> = turns.iter().take(20).map(|t| t.body.as_str()).collect();
    assert_eq!(turns.len(), 25);
    let bodies: Vec<_> = sessions.iter().map(|s| s.body.clone()).collect();
    assert_eq!(bodies, [want.join("\n")]);
    assert_eq!(sessions.first().map(|s| s.ts), Some(NOW.saturating_add(25)));
    Ok(())
}

#[test]
fn the_roll_up_is_the_same_bytes_every_run() -> Test {
    let mut bodies = Vec::new();
    for n in 0..2 {
        let scratch = Scratch::new(&format!("rollupsame-{n}"))?;
        session_of(&scratch, "s", 3);
        bodies.push(summaries(&scratch, Scope::Session)?);
    }
    assert_eq!(bodies.first(), bodies.last());
    Ok(())
}

#[test]
fn a_session_without_turns_has_no_roll_up() -> Test {
    let scratch = Scratch::new("noturns")?;
    let out = scratch.run(Kind::SessionEnd, &fixture(Kind::SessionEnd)?);
    assert_eq!(out.note, None);
    assert!(summaries(&scratch, Scope::Session)?.is_empty());
    assert_eq!(scratch.observations()?.len(), 1);
    Ok(())
}

#[test]
fn rollup_joins_at_most_20_bodies_by_newline() {
    let turn = |n: usize| Summary {
        ts: 0,
        session: "s".to_owned(),
        scope: Scope::Turn,
        ide: "claude-code".to_owned(),
        body: format!("t{n}"),
    };
    let turns: Vec<_> = (0..30).map(turn).collect();
    let want: Vec<_> = (0..TURNS).map(|n| format!("t{n}")).collect();
    assert_eq!(rollup(&turns), want.join("\n"));
    assert_eq!(rollup(&[]), "");
}
