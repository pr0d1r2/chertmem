use std::path::Path;

use rusqlite::{Connection, params};

use crate::grammar::{Intensity, compress};
use crate::import::cavemem::tests::fixture;
use crate::import::tests::{NOW, Res, Scratch, Test};
use crate::import::{Error, Imported, cavemem};
use crate::store::{Scope, Store, Summary};

/// cavemem's `insertSummary` statement, verbatim from the bundle the
/// fixture schema comes from (root R1).
const SUMMARY: &str = "INSERT INTO summaries(session_id, scope, content, compressed, intensity, ts) VALUES (?, ?, ?, ?, ?, ?)";

/// The fixture db at `path` plus 4 summaries: a turn written twice, a
/// session summary with a private span, an uncompressed turn.
fn noted(path: &Path) -> Res<Connection> {
    let conn = fixture(path)?;
    let rows = [
        ("s-a", "turn", "fixed build", 1, 40),
        ("s-a", "turn", "fixed build", 1, 40),
        ("s-a", "session", "all <private>pw</private>done", 1, 50),
        ("s-b", "turn", "Please add the tests", 0, 60),
    ];
    for (s, scope, content, compressed, at) in rows {
        let ts = NOW.saturating_add(at);
        let args = params![s, scope, content, compressed, "full", ts];
        conn.execute(SUMMARY, args)?;
    }
    Ok(conn)
}

/// Import the summary fixture at `s`'s `cavemem.db` (built once).
fn imported(s: &Scratch) -> Res<Imported> {
    let db = s.path("cavemem.db");
    if !db.exists() {
        noted(&db)?;
    }
    Ok(cavemem(&s.config, &s.root(), &db)?)
}

/// A summary `at` ms after [`NOW`].
fn note(at: u64, session: &str, scope: Scope, body: &str) -> Summary {
    let ide = if session == "s-a" {
        "claude-code"
    } else {
        "cursor"
    };
    Summary {
        ts: NOW.saturating_add(at),
        session: session.to_owned(),
        scope,
        ide: ide.to_owned(),
        body: body.to_owned(),
    }
}

/// Every summary the fixture should leave, turns then sessions.
fn wanted() -> Vec<Summary> {
    let added = compress("Please add the tests", Intensity::Full);
    vec![
        note(40, "s-a", Scope::Turn, "fixed build"),
        note(40, "s-a", Scope::Turn, "fixed build"),
        note(60, "s-b", Scope::Turn, &added),
        note(50, "s-a", Scope::Session, "all done"),
    ]
}

/// Every summary in `store`, turns then sessions.
fn held(store: &mut Store) -> Res<Vec<Summary>> {
    let mut all = store.summaries(None, Scope::Turn)?;
    all.extend(store.summaries(None, Scope::Session)?);
    Ok(all)
}

#[test]
fn every_summary_lands_with_its_session_scope_and_ide() -> Test {
    let s = Scratch::new("sum-rows")?;
    let mut store = s.store()?;
    let got = imported(&s)?;
    assert_eq!((got.observations, got.summaries), (3, 4));
    assert_eq!(held(&mut store)?, wanted());
    let a = store.summaries(Some("s-a"), Scope::Turn)?;
    assert_eq!(a.len(), 2);
    Ok(())
}

#[test]
fn importing_twice_keeps_the_same_summaries() -> Test {
    let s = Scratch::new("sum-twice")?;
    let mut store = s.store()?;
    imported(&s)?;
    assert_eq!(imported(&s)?.summaries, 4);
    assert_eq!(held(&mut store)?, wanted());
    Ok(())
}

#[test]
fn a_summary_written_once_more_since_lands_once() -> Test {
    let s = Scratch::new("sum-more")?;
    let mut store = s.store()?;
    let conn = noted(&s.path("cavemem.db"))?;
    imported(&s)?;
    let ts = NOW.saturating_add(40);
    let again = params!["s-a", "turn", "fixed build", 1, "full", ts];
    conn.execute(SUMMARY, again)?;
    imported(&s)?;
    let turns = store.summaries(Some("s-a"), Scope::Turn)?;
    assert_eq!(turns.len(), 3);
    Ok(())
}

/// The error importing the summary fixture after `sql` gives.
fn refused_after(name: &str, sql: &str) -> Res<Error> {
    let s = Scratch::new(name)?;
    let db = s.path("cavemem.db");
    noted(&db)?.execute_batch(sql)?;
    let got = cavemem(&s.config, &s.root(), &db);
    got.err().ok_or_else(|| "import succeeded".into())
}

#[test]
fn an_unknown_scope_is_refused() -> Test {
    let sql = "PRAGMA ignore_check_constraints = ON;
      UPDATE summaries SET scope = 'week' WHERE id = 1;";
    let e = refused_after("sum-scope", sql)?;
    assert!(e.to_string().contains("week"), "{e}");
    Ok(())
}

#[test]
fn a_negative_summary_timestamp_is_refused() -> Test {
    let sql = "UPDATE summaries SET ts = -7 WHERE id = 2;";
    let e = refused_after("sum-negative", sql)?;
    assert!(e.to_string().contains("-7"), "{e}");
    Ok(())
}
