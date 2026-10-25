use std::error::Error as StdError;
use std::fs;
use std::path::PathBuf;

use crate::config::{Config, Dimension};
use crate::store::tests::{Scratch, Test, WEEK, chunk};
use crate::store::{
    Error, Filter, Listed, Mode, Scope, Store, Summary, Written,
};

fn summary(ts: u64, session: &str, scope: Scope, body: &str) -> Summary {
    Summary {
        ts,
        session: session.into(),
        scope,
        ide: "claude-code".into(),
        body: body.into(),
    }
}

fn bodies(found: &[Summary]) -> Vec<&str> {
    found.iter().map(|s| s.body.as_str()).collect()
}

/// 3 turns of s1 over 2 buckets, out of order, 1 of s2, 1 session.
fn fill(store: &mut Store) -> Test {
    let rows = [
        summary(WEEK + 1, "s1", Scope::Turn, "turn 3"),
        summary(5, "s1", Scope::Turn, "turn 1"),
        summary(6, "s2", Scope::Turn, "other session"),
        summary(7, "s1", Scope::Turn, "turn 2"),
        summary(WEEK + 2, "s1", Scope::Session, "whole s1"),
    ];
    for r in &rows {
        assert_eq!(store.append_summary(r)?, Written::Stored);
    }
    Ok(())
}

#[test]
fn summaries_come_back_by_session_and_scope_in_time_order() -> Test {
    let s = Scratch::new("sum-order")?;
    let mut store = s.open(Mode::Read)?;
    fill(&mut store)?;
    let turns = store.summaries(Some("s1"), Scope::Turn)?;
    assert_eq!(bodies(&turns), ["turn 1", "turn 2", "turn 3"]);
    let whole = store.summaries(Some("s1"), Scope::Session)?;
    assert_eq!(whole, [summary(WEEK + 2, "s1", Scope::Session, "whole s1")]);
    let all = store.summaries(None, Scope::Turn)?;
    assert_eq!(
        bodies(&all),
        ["turn 1", "other session", "turn 2", "turn 3"]
    );
    Ok(())
}

#[test]
fn a_summary_lands_in_the_bucket_of_its_own_time() -> Test {
    let s = Scratch::new("sum-bucket")?;
    let mut store = s.open(Mode::Read)?;
    store.append_summary(&summary(WEEK + 1, "s1", Scope::Turn, "b"))?;
    let path = format!("chunks/{}/19700108-7d.db", store.project());
    let sql = "SELECT body FROM summaries";
    let body: String = s.sql(&path)?.query_row(sql, [], |r| r.get(0))?;
    assert_eq!(body, "b");
    Ok(())
}

#[test]
fn another_projects_summaries_stay_out() -> Test {
    let s = Scratch::new("sum-project")?;
    let mut other = s.open_other("elsewhere")?;
    other.append_summary(&summary(5, "s1", Scope::Turn, "theirs"))?;
    let mut store = s.open(Mode::Read)?;
    assert!(store.summaries(Some("s1"), Scope::Turn)?.is_empty());
    Ok(())
}

/// Spool `r` under a held catalog lock; its spool file.
fn spool(s: &Scratch, r: &Summary) -> Result<PathBuf, Box<dyn StdError>> {
    drop(s.open(Mode::Read)?);
    let held = s.sql("catalog.db")?;
    held.execute_batch("BEGIN IMMEDIATE")?;
    let mut hook = s.open(Mode::Write)?;
    assert_eq!(hook.append_summary(r)?, Written::Spooled);
    drop(held);
    let spool = s.data().join("spool");
    let file = fs::read_dir(&spool)?.next().ok_or("nothing spooled")??;
    Ok(file.path())
}

#[test]
fn a_held_lock_spools_a_summary_until_a_reading_open() -> Test {
    let s = Scratch::new("sum-spool")?;
    let r = summary(5, "s1", Scope::Session, "rolled up");
    let file = spool(&s, &r)?;
    let copy = fs::read(&file)?;
    let mut store = s.open(Mode::Read)?;
    let once = store.summaries(Some("s1"), Scope::Session)?;
    assert_eq!(once, std::slice::from_ref(&r));
    // Replayed twice, as 2 readers may: still 1 summary.
    fs::write(&file, copy)?;
    store.replay()?;
    assert_eq!(store.summaries(Some("s1"), Scope::Session)?, [r]);
    Ok(())
}

#[test]
fn summary_writes_check_time_and_reads_need_a_reading_open() -> Test {
    let s = Scratch::new("sum-guards")?;
    let mut hook = s.open(Mode::Write)?;
    let late = summary(1 << 48, "s1", Scope::Turn, "x");
    assert_eq!(
        hook.append_summary(&late),
        Err(Error::BadTime { ts: 1 << 48 })
    );
    let read = hook.summaries(None, Scope::Turn).err();
    assert_eq!(read, Some(Error::WriteOnly));
    Ok(())
}

#[test]
fn scopes_round_trip_through_their_names() {
    for scope in [Scope::Turn, Scope::Session] {
        assert_eq!(Scope::parse(scope.as_str()), Some(scope));
    }
    assert_eq!(Scope::parse("week"), None);
}

fn listed(found: &[Listed]) -> Vec<&str> {
    found.iter().map(|l| l.summary.body.as_str()).collect()
}

#[test]
fn summaries_in_reads_1_partition_by_time_then_id() -> Test {
    let s = Scratch::new("sum-in")?;
    let mut store = s.open(Mode::Read)?;
    fill(&mut store)?;
    store.append_summary(&summary(6, "s3", Scope::Turn, "tied"))?;
    let turns =
        store.summaries_in(&chunk(&store, "19700101-7d"), Scope::Turn)?;
    let bodies = listed(&turns);
    let ends = (bodies.first(), bodies.last(), bodies.len());
    assert_eq!(ends, (Some(&"turn 1"), Some(&"turn 2"), 4));
    let keys: Vec<(u64, &str)> = turns
        .iter()
        .map(|l| (l.summary.ts, l.id.as_str()))
        .collect();
    assert!(keys.windows(2).all(|w| w.first() < w.last()), "{keys:?}");
    Ok(())
}

#[test]
fn summaries_in_reads_each_scope_and_nothing_where_no_file_is() -> Test {
    let s = Scratch::new("sum-in-scope")?;
    let mut store = s.open(Mode::Read)?;
    fill(&mut store)?;
    let week1 = chunk(&store, "19700108-7d");
    assert_eq!(
        listed(&store.summaries_in(&week1, Scope::Turn)?),
        ["turn 3"]
    );
    let whole = store.summaries_in(&week1, Scope::Session)?;
    let whole: Vec<Summary> = whole.into_iter().map(|l| l.summary).collect();
    assert_eq!(whole, [summary(WEEK + 2, "s1", Scope::Session, "whole s1")]);
    let gone = chunk(&store, "19700115-7d");
    assert!(store.summaries_in(&gone, Scope::Turn)?.is_empty());
    Ok(())
}

/// A config without the project dim, so projects share files, and a
/// summary of another project already stored under it.
fn theirs_beside(s: &Scratch) -> Result<Config, Box<dyn StdError>> {
    let mut cfg = s.config()?;
    cfg.partition.by = vec![Dimension::Time];
    let other = s.project().with_file_name("elsewhere");
    fs::create_dir_all(&other)?;
    let mut theirs = Store::open(&cfg, &other, Mode::Read)?;
    theirs.append_summary(&summary(6, "s2", Scope::Turn, "theirs"))?;
    Ok(cfg)
}

#[test]
fn summaries_in_keeps_to_its_project_and_a_reading_open() -> Test {
    let s = Scratch::new("sum-in-project")?;
    let cfg = theirs_beside(&s)?;
    let mut store = Store::open(&cfg, &s.project(), Mode::Read)?;
    store.append_summary(&summary(5, "s1", Scope::Turn, "mine"))?;
    let all = store.partitions(&Filter::default())?;
    let [p] = all.as_slice() else {
        return Err(format!("{all:?}").into());
    };
    assert_eq!(listed(&store.summaries_in(&p.path, Scope::Turn)?), ["mine"]);
    let mut hook = Store::open(&cfg, &s.project(), Mode::Write)?;
    let refused = hook.summaries_in(&p.path, Scope::Turn);
    assert_eq!(refused, Err(Error::WriteOnly));
    Ok(())
}

/// What `EXPLAIN QUERY PLAN` says of [`super::list_sql`] for `session`.
fn plan(session: Option<&str>) -> Result<String, Box<dyn StdError>> {
    let s = Scratch::new(&format!("sum-plan-{}", session.is_some()))?;
    let mut store = s.open(Mode::Read)?;
    store.append_summary(&summary(5, "s1", Scope::Turn, "b"))?;
    let conn = s.sql(&chunk(&store, "19700101-7d"))?;
    let sql = format!("EXPLAIN QUERY PLAN {}", super::list_sql(session));
    let mut stmt = conn.prepare(&sql)?;
    let args = rusqlite::params!["p", session, "turn"];
    let rows = stmt.query_map(args, |r| r.get::<_, String>(3))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?.join("; "))
}

#[test]
fn listing_summaries_searches_an_index_with_or_without_a_session() -> Test {
    let every = plan(None)?;
    let index = "summaries_scope (project=? AND scope=?)";
    assert!(every.contains(index), "{every}");
    let one = plan(Some("s1"))?;
    let index = "summaries_session (project=? AND session=? AND scope=?)";
    assert!(one.contains(index), "{one}");
    Ok(())
}
