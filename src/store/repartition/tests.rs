use std::error::Error as StdError;

use crate::config::{Config, Dimension, Span, Unit};
use crate::store::tests::{Scratch, Test, WEEK, chunk, event, stored};
use crate::store::{
    Appended, Error, Filter, Lookup, Mode, Observation, Ref, Scope, Store,
    Summary,
};

const DAY: u64 = 86_400_000;

/// Daily buckets in 2 shards: every default-spec file moves.
fn daily(s: &Scratch) -> Result<Config, Box<dyn StdError>> {
    let mut cfg = s.config()?;
    cfg.partition.time_bucket = Span {
        amount: 1,
        unit: Unit::Days,
    };
    cfg.partition.shards = 2;
    Ok(cfg)
}

fn open(s: &Scratch, cfg: &Config) -> Result<Store, Box<dyn StdError>> {
    Ok(Store::open(cfg, &s.project(), Mode::Read)?)
}

fn turn(ts: u64, body: &str) -> Summary {
    Summary {
        ts,
        session: "session-1".into(),
        scope: Scope::Turn,
        ide: "claude-code".into(),
        body: body.into(),
    }
}

/// 4 rows over 3 days and 2 weeks, a turn summary; the refs.
fn fill(store: &mut Store) -> Result<Vec<Ref>, Box<dyn StdError>> {
    let mut refs = Vec::new();
    for (ts, id) in [(5, "t1"), (DAY + 1, "t2"), (WEEK + 1, "t3"), (6, "t4")] {
        refs.push(stored(store.append(&event(ts, Some(id), id))?)?);
    }
    store.append_summary(&turn(7, "turn 1"))?;
    Ok(refs)
}

/// What `refs` resolve to; an error for a dropped one.
fn rows(
    store: &mut Store,
    refs: &[Ref],
) -> Result<Vec<Observation>, Box<dyn StdError>> {
    let found = store.get(refs)?.into_iter().map(|l| match l {
        Lookup::Found(o) => Ok(o),
        Lookup::Dropped(r) => Err(format!("{r} dropped")),
    });
    Ok(found.collect::<Result<_, _>>()?)
}

/// Fill under the defaults; the refs and the rows they resolve to.
fn before(
    s: &Scratch,
) -> Result<(Vec<Ref>, Vec<Observation>), Box<dyn StdError>> {
    let mut old = s.open(Mode::Read)?;
    let refs = fill(&mut old)?;
    let rows = rows(&mut old, &refs)?;
    Ok((refs, rows))
}

#[test]
fn every_ref_resolves_to_its_own_row_after_repartition() -> Test {
    let s = Scratch::new("repart-refs")?;
    let (refs, was) = before(&s)?;
    let mut new = open(&s, &daily(&s)?)?;
    new.repartition()?;
    assert_eq!(rows(&mut new, &refs)?, was);
    let mut all = new.observations(&Filter::default())?;
    all.sort_by(|a, b| a.reference.cmp(&b.reference));
    let mut want = was;
    want.sort_by(|a, b| a.reference.cmp(&b.reference));
    assert_eq!(all, want);
    assert!(new.generation()? > 0);
    Ok(())
}

#[test]
fn old_files_go_and_every_partition_has_the_current_spec() -> Test {
    let s = Scratch::new("repart-spec")?;
    before(&s)?;
    let mut new = open(&s, &daily(&s)?)?;
    new.repartition()?;
    let spec = "by=project,time;time_bucket=1d;shards=2";
    for p in new.partitions(&Filter::default())? {
        assert_eq!(p.spec, spec, "{}", p.path);
        assert!(s.data().join(&p.path).is_file(), "{}", p.path);
    }
    assert!(!s.data().join(chunk(&new, "19700101-7d")).exists());
    Ok(())
}

#[test]
fn identities_and_summaries_survive_repartition() -> Test {
    let s = Scratch::new("repart-identity")?;
    let (refs, _) = before(&s)?;
    let mut new = open(&s, &daily(&s)?)?;
    new.repartition()?;
    let again = new.append(&event(9, Some("t1"), "again"))?;
    let first = refs.first().cloned().ok_or("no refs")?;
    assert_eq!(again, Appended::Stored(first));
    let turns = new.summaries(None, Scope::Turn)?;
    assert_eq!(turns, [turn(7, "turn 1")]);
    Ok(())
}

#[test]
fn a_cold_partition_is_rewritten_too() -> Test {
    let s = Scratch::new("repart-cold")?;
    let mut old = s.open(Mode::Read)?;
    let refs = fill(&mut old)?;
    let was = rows(&mut old, &refs)?;
    let path = chunk(&old, "19700101-7d");
    old.compact(&path)?;
    let mut new = open(&s, &daily(&s)?)?;
    new.repartition()?;
    assert_eq!(rows(&mut new, &refs)?, was);
    assert!(!s.data().join(format!("{path}.zst")).exists());
    Ok(())
}

#[test]
fn rows_whose_file_stays_keep_it_under_the_new_spec() -> Test {
    let s = Scratch::new("repart-stay")?;
    let (refs, was) = before(&s)?;
    let mut cfg = s.config()?;
    cfg.partition.by = vec![Dimension::Time, Dimension::Project];
    let mut new = open(&s, &cfg)?;
    new.repartition()?;
    let all = new.partitions(&Filter::default())?;
    assert_eq!(all.len(), 2);
    assert!(all.iter().all(|p| p.dims == "time,project"));
    assert_eq!(rows(&mut new, &refs)?, was);
    Ok(())
}

#[test]
fn a_sealed_target_refuses_and_moves_nothing() -> Test {
    let s = Scratch::new("repart-sealed")?;
    let (refs, was) = before(&s)?;
    let mut cfg = daily(&s)?;
    cfg.partition.shards = 1;
    let mut new = open(&s, &cfg)?;
    new.append(&event(8, None, "day 0 under the new spec"))?;
    new.seal(&chunk(&new, "19700101-1d"))?;
    let refused = new.repartition();
    assert!(
        matches!(refused, Err(Error::ReadOnly { .. })),
        "{refused:?}"
    );
    assert_eq!(rows(&mut new, &refs)?, was);
    Ok(())
}

fn shards(s: &Scratch, n: u32) -> Result<Config, Box<dyn StdError>> {
    let mut cfg = s.config()?;
    cfg.partition.shards = n;
    Ok(cfg)
}

/// 24 rows and 8 turns in week 0 over 2 shards; the refs.
fn sharded(s: &Scratch) -> Result<Vec<Ref>, Box<dyn StdError>> {
    let mut old = open(s, &shards(s, 2)?)?;
    let mut refs = Vec::new();
    for ts in 0..24 {
        refs.push(stored(old.append(&event(ts, None, &ts.to_string()))?)?);
    }
    for ts in 0..8 {
        old.append_summary(&turn(ts, &ts.to_string()))?;
    }
    Ok(refs)
}

#[test]
fn shards_2_to_3_keep_some_rows_in_place_and_move_the_rest() -> Test {
    let s = Scratch::new("repart-shards")?;
    let refs = sharded(&s)?;
    let mut new = open(&s, &shards(&s, 3)?)?;
    new.repartition()?;
    let all = new.partitions(&Filter::default())?;
    assert_eq!(all.len(), 3);
    let total = all.iter().map(|p| p.stats.rows).sum::<u64>();
    assert_eq!(total, 24);
    assert_eq!(rows(&mut new, &refs)?.len(), 24);
    assert_eq!(new.summaries(None, Scope::Turn)?.len(), 8);
    assert_eq!(new.observations(&Filter::default())?.len(), 24);
    Ok(())
}

#[test]
fn a_source_whose_file_is_gone_is_forgotten() -> Test {
    let s = Scratch::new("repart-gone")?;
    let (refs, _) = before(&s)?;
    let week0 = chunk(&s.open(Mode::Read)?, "19700101-7d");
    std::fs::remove_file(s.data().join(&week0))?;
    let mut new = open(&s, &daily(&s)?)?;
    new.repartition()?;
    let lost = refs.first().cloned().ok_or("no refs")?;
    let found = new.get(std::slice::from_ref(&lost))?;
    assert_eq!(found, [Lookup::Dropped(lost)]);
    Ok(())
}

#[test]
fn nothing_to_rewrite_leaves_generation_alone() -> Test {
    let s = Scratch::new("repart-noop")?;
    before(&s)?;
    let mut store = s.open(Mode::Read)?;
    let was = store.generation()?;
    store.repartition()?;
    assert_eq!(store.generation()?, was);
    Ok(())
}
