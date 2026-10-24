use std::path::Path;

use rusqlite::{Connection, params};

use crate::import::cavemem::tests::{OBSERVATION, SESSION, fixture};
use crate::import::tests::{NOW, Res, Scratch, Test};
use crate::import::{Imported, cavemem};
use crate::store::{Filter, Mode, Scope, Store, project_id};

/// cavemem's `insertSummary` statement, verbatim from the bundle the
/// fixture schema comes from (root R1).
const SUMMARY: &str = "INSERT INTO summaries(session_id, scope, content, compressed, intensity, ts) VALUES (?, ?, ?, ?, ?, ?)";

/// The fixture (2 sessions, null cwd) plus s-c in `a`, s-d in `b` and
/// s-e with an empty cwd, 1 observation each; a turn for s-c and s-e.
fn spread(path: &Path, a: &str, b: &str) -> Res<Connection> {
    let conn = fixture(path)?;
    let ts = NOW.saturating_add(100);
    for (id, cwd) in [("s-c", a), ("s-d", b), ("s-e", "")] {
        let session = params![id, "claude-code", cwd, NOW, None::<String>];
        conn.execute(SESSION, session)?;
        let body = format!("in {id}");
        let row = params![id, "stop", body, 1, "full", ts, None::<String>];
        conn.execute(OBSERVATION, row)?;
    }
    for id in ["s-c", "s-e"] {
        let note = params![id, "turn", format!("sum {id}"), 1, "full", ts];
        conn.execute(SUMMARY, note)?;
    }
    Ok(conn)
}

/// A scratch dir with 2 project dirs and the spread fixture.
fn scratch(name: &str) -> Res<Scratch> {
    let s = Scratch::new(name)?;
    for dir in ["a", "b"] {
        std::fs::create_dir_all(s.path(dir))?;
    }
    let (a, b) = (s.path("a"), s.path("b"));
    let text = |p: &Path| p.to_str().map(str::to_owned).ok_or("not utf-8");
    spread(&s.path("cavemem.db"), &text(&a)?, &text(&b)?)?;
    Ok(s)
}

fn imported(s: &Scratch) -> Res<Imported> {
    Ok(cavemem(&s.config, &s.root(), &s.path("cavemem.db"))?)
}

/// The sessions of every observation and turn in `root`'s project.
fn held(s: &Scratch, root: &Path) -> Res<(Vec<String>, Vec<String>)> {
    let mut store = Store::open(&s.config, root, Mode::Read)?;
    let mine = Filter {
        project: Some(project_id(root)),
        ..Filter::default()
    };
    let rows = store.observations(&mine)?.into_iter().map(|o| o.session);
    let turns = store.summaries(None, Scope::Turn)?.into_iter();
    Ok((rows.collect(), turns.map(|t| t.session).collect()))
}

/// `names` as owned strings.
fn owned(names: &[&str]) -> Vec<String> {
    names.iter().map(|&n| n.to_owned()).collect()
}

/// Whether each project holds just its own sessions' rows.
fn split_by_cwd(s: &Scratch) -> Res<bool> {
    let here = (owned(&["s-a", "s-a", "s-b", "s-e"]), owned(&["s-e"]));
    let a = (owned(&["s-c"]), owned(&["s-c"]));
    let b = (owned(&["s-d"]), Vec::new());
    Ok(held(s, &s.root())? == here
        && held(s, &s.path("a"))? == a
        && held(s, &s.path("b"))? == b)
}

#[test]
fn each_session_lands_in_the_project_of_its_cwd() -> Test {
    let s = scratch("cm-split")?;
    let got = imported(&s)?;
    let want = Imported {
        observations: 6,
        summaries: 2,
        no_cwd: 5,
    };
    assert_eq!(got, want);
    assert!(split_by_cwd(&s)?);
    Ok(())
}

#[test]
fn a_split_import_twice_keeps_the_same_rows() -> Test {
    let s = scratch("cm-split-twice")?;
    imported(&s)?;
    imported(&s)?;
    assert!(split_by_cwd(&s)?);
    let all = s.store()?.observations(&Filter::default())?;
    assert_eq!(all.len(), 6);
    Ok(())
}
