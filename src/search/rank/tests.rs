use std::error::Error as StdError;

use super::{Rater, Scored, Stats, order, score, top};
use crate::search::tests::Test;
use crate::store::{Observation, Ref};

fn terms(t: &[&str]) -> Vec<String> {
    t.iter().map(|&w| w.to_owned()).collect()
}

fn stats(docs: u64, tokens: u64, df: &[u64]) -> Stats {
    Stats {
        docs,
        tokens,
        df: df.to_vec(),
    }
}

fn row(
    id: &str,
    ts: u64,
    body: &str,
) -> Result<Observation, Box<dyn StdError>> {
    Ok(Observation {
        reference: Ref::parse(&format!("chert:{id}"))?,
        id: id.into(),
        project: "p".into(),
        ts,
        session: "s".into(),
        ide: "claude-code".into(),
        kind: "post-tool-use".into(),
        identity: None,
        body: body.into(),
    })
}

#[test]
fn no_term_scores_0_and_a_term_scores_above_0() {
    let st = stats(10, 40, &[2]);
    assert!(score(&st, &terms(&["cargo"]), "git push").abs() < f64::EPSILON);
    assert!(score(&st, &terms(&["cargo"]), "cargo build") > 0.0);
}

#[test]
fn a_rarer_term_more_often_in_a_shorter_row_scores_higher() {
    let t = terms(&["cargo"]);
    let base = score(&stats(10, 40, &[5]), &t, "cargo a b c");
    assert!(score(&stats(10, 40, &[1]), &t, "cargo a b c") > base);
    assert!(score(&stats(10, 40, &[5]), &t, "cargo cargo b c") > base);
    assert!(score(&stats(10, 40, &[5]), &t, "cargo") > base);
}

#[test]
fn a_term_in_every_row_still_counts_and_no_stats_stay_finite() {
    let t = terms(&["cargo"]);
    assert!(score(&stats(3, 9, &[3]), &t, "cargo x y") > 0.0);
    let empty = score(&stats(0, 0, &[0]), &t, "cargo");
    assert!(empty.is_finite() && empty > 0.0, "{empty}");
}

#[test]
fn the_same_row_gets_the_same_bits() {
    let (st, t) = (stats(7, 30, &[2, 3]), terms(&["a", "b"]));
    let once = score(&st, &t, "a b b c");
    assert_eq!(once.to_bits(), score(&st, &t, "a b b c").to_bits());
}

#[test]
fn order_is_score_desc_then_ts_desc_then_id_desc() -> Test {
    let scored = |s: f64, r: Observation| Scored { score: s, row: r };
    let rows = vec![
        scored(1.0, row("0000000A", 5, "x")?),
        scored(2.0, row("0000000B", 1, "x")?),
        scored(1.0, row("0000000C", 9, "x")?),
        scored(1.0, row("0000000D", 5, "x")?),
    ];
    let ids: Vec<String> =
        top(rows.clone(), 9).into_iter().map(|s| s.row.id).collect();
    assert_eq!(ids, ["0000000B", "0000000C", "0000000D", "0000000A"]);
    assert_eq!(top(rows.clone(), 1).len(), 1);
    let (a, b) = (rows.first().ok_or("a")?, rows.get(3).ok_or("d")?);
    assert_eq!(order(a, b), std::cmp::Ordering::Greater);
    Ok(())
}

fn rater() -> Rater {
    Rater {
        stats: stats(4, 8, &[1]),
        terms: terms(&["cargo"]),
        kind: None,
        session: None,
        keep: 1,
    }
}

#[test]
fn a_rater_refuses_other_kinds_sessions_and_misses() -> Test {
    let mut r = rater();
    let cargo = || row("0000000A", 1, "cargo");
    assert!(r.rate(row("0000000A", 1, "git")?).is_none());
    assert!(r.rate(cargo()?).is_some());
    r.kind = Some("stop".into());
    assert!(r.rate(cargo()?).is_none());
    r.kind = None;
    r.session = Some("other".into());
    assert!(r.rate(cargo()?).is_none());
    r.session = Some("s".into());
    assert!(r.rate(cargo()?).is_some());
    Ok(())
}

#[test]
fn a_rater_keeps_its_best_only() -> Test {
    let rows = vec![row("0000000A", 1, "cargo")?, row("0000000B", 2, "cargo")?];
    let best = rater().best(rows);
    assert_eq!(best.iter().map(|s| s.row.ts).collect::<Vec<_>>(), [2]);
    Ok(())
}
