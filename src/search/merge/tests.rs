use std::error::Error as StdError;
use std::fs;
use std::path::{Path, PathBuf};

use super::{Job, parallel, split, workers};
use crate::config::{Config, Dimension, Span, Unit};
use crate::search::rank::{Rater, top};
use crate::search::terms::{Stats, Terms};
use crate::search::tests::{BASE, DAY, Scratch, Test, engine, event, fresh};
use crate::search::{Hit, Query};
use crate::store::{Filter, Mode, Store, project_id};

fn paths(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("p{i}")).collect()
}

fn rater(stats: Stats, keep: usize) -> Rater {
    Rater {
        stats,
        terms: vec!["cargo".into()],
        kind: None,
        session: None,
        keep,
    }
}

#[test]
fn split_deals_every_path_once_and_no_group_empty() {
    let groups = split(&paths(7), 3);
    assert_eq!(groups.len(), 3);
    let mut all: Vec<String> = groups.concat();
    all.sort();
    assert_eq!(all, paths(7));
    assert_eq!(split(&paths(2), 3).len(), 2);
    assert!(split(&[], 1).is_empty());
}

#[test]
fn workers_are_at_least_1_and_at_most_1_per_partition() {
    assert_eq!(workers(0), 1);
    assert_eq!(workers(1), 1);
    assert!(workers(64) >= 1 && workers(64) <= 64);
}

/// 12 rows, 1 per weekly partition, every 3rd scoring higher.
fn twelve(store: &mut Store) -> Result<Vec<String>, Box<dyn StdError>> {
    let mut ts = BASE;
    for i in 0..12 {
        let body = if i % 3 == 0 {
            "cargo cargo x"
        } else {
            "cargo test"
        };
        store.append(&event(ts, "s", body))?;
        ts = ts.saturating_add(8 * DAY);
    }
    let parts = store.partitions(&Filter::default())?;
    Ok(parts.into_iter().map(|p| p.path).collect())
}

#[test]
fn the_parallel_best_equal_the_best_of_all_rows() -> Test {
    let (s, cfg, mut store) = fresh("merge-all")?;
    let paths = twelve(&mut store)?;
    assert_eq!(paths.len(), 12);
    let mut terms = Terms::open(&s.data())?;
    terms.refresh(&mut store)?;
    let rater = rater(terms.stats(&["cargo".into()])?, 5);
    let (filter, root) = (Filter::default(), s.root());
    let got = parallel(&job(&cfg, &root, &filter, &rater), &paths)?;
    let all = store.observations(&filter)?;
    let one = top(all.into_iter().filter_map(|r| rater.rate(r)).collect(), 5);
    assert_eq!((got.len(), got), (5, one));
    Ok(())
}

fn job<'a>(
    cfg: &'a Config,
    root: &'a Path,
    filter: &'a Filter,
    rater: &'a Rater,
) -> Job<'a> {
    Job {
        cfg,
        root,
        filter,
        rater,
    }
}

#[test]
fn a_worker_that_cannot_open_its_store_fails_the_query() -> Test {
    let s = Scratch::new("merge-fail")?;
    let mut cfg = s.config()?;
    cfg.storage.data_dir = None;
    let empty = Stats {
        docs: 0,
        tokens: 0,
        df: vec![0],
    };
    let (filter, root, rater) = (Filter::default(), s.root(), rater(empty, 1));
    let found = parallel(&job(&cfg, &root, &filter, &rater), &paths(2));
    assert!(found.is_err());
    Ok(())
}

const VOCAB: [&str; 8] = [
    "cargo", "build", "test", "git", "push", "fix", "nix", "spec",
];

/// splitmix64: a seeded, repeatable stream, no crate.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: u64) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ z.wrapping_shr(30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ z.wrapping_shr(27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        (z ^ z.wrapping_shr(31)).checked_rem(n).unwrap_or(0)
    }

    fn words(&mut self, max: u64) -> String {
        let n = self.below(max).saturating_add(1);
        let pick = |r: &mut Self| usize::try_from(r.below(8)).unwrap_or(0);
        let words = (0..n).map(|_| VOCAB.get(pick(self)).copied());
        let words: Vec<&str> = words.map(Option::unwrap_or_default).collect();
        words.join(" ")
    }

    fn maybe(&mut self) -> bool {
        self.below(2) == 0
    }
}

/// Rows `(project, ts, session, body)`, every ts distinct.
type Rows = Vec<(usize, u64, String, String)>;

fn fixture(rng: &mut Rng) -> Rows {
    let mut ts = BASE;
    let mut rows = Vec::new();
    for _ in 0..rng.below(30).saturating_add(20) {
        ts = ts.saturating_add(rng.below(2 * DAY).saturating_add(1));
        let project = usize::from(rng.maybe());
        let session = format!("s{}", rng.below(3));
        rows.push((project, ts, session, rng.words(6)));
    }
    rows
}

/// 1 data dir holding the fixture's rows in 2 projects.
struct Side {
    s: Scratch,
    cfg: Config,
    roots: [PathBuf; 2],
}

fn side(
    name: &str,
    rows: &Rows,
    tweak: fn(&mut Config),
) -> Result<Side, Box<dyn StdError>> {
    let s = Scratch::new(name)?;
    let mut cfg = s.config()?;
    tweak(&mut cfg);
    let roots = [s.root(), s.root().join("other")];
    for root in &roots {
        fs::create_dir_all(root)?;
    }
    for (p, ts, session, body) in rows {
        let mut store =
            Store::open(&cfg, roots.get(*p).ok_or("project")?, Mode::Read)?;
        store.append(&event(*ts, session, body))?;
    }
    Ok(Side { s, cfg, roots })
}

/// Weekly partitions over 3 shards, and 1 partition of every row.
fn sides(seed: u64, rows: &Rows) -> Result<(Side, Side), Box<dyn StdError>> {
    let split =
        side(&format!("v2-a-{seed}"), rows, |c| c.partition.shards = 3)?;
    let one = side(&format!("v2-b-{seed}"), rows, |c| {
        c.partition.by = vec![Dimension::Time];
        c.partition.time_bucket = Span {
            amount: 120,
            unit: Unit::Months,
        };
    })?;
    Ok((split, one))
}

fn parts(d: &Side) -> Result<usize, Box<dyn StdError>> {
    Ok(d.s.open(&d.cfg)?.partitions(&Filter::default())?.len())
}

fn random_query(rng: &mut Rng, side: &Side) -> Query {
    let project = side.roots.get(usize::try_from(rng.below(3)).unwrap_or(0));
    let absent = if rng.maybe() { " absent" } else { "" };
    let limit = u32::try_from(rng.below(8)).unwrap_or(0).saturating_add(1);
    Query {
        text: format!("{}{absent}", rng.words(3)),
        limit: Some(limit),
        since: rng
            .maybe()
            .then(|| BASE.saturating_add(rng.below(40 * DAY))),
        project: project.map(|r| project_id(r)),
        session: rng.maybe().then(|| format!("s{}", rng.below(3))),
        ..Query::default()
    }
}

/// What 2 stores of the same rows agree on: refs differ, every ts is
/// distinct.
fn key(hits: &[Hit]) -> Vec<(u64, u64, String, String)> {
    let one = |h: &Hit| {
        (
            h.score.to_bits(),
            h.ts,
            h.session.clone(),
            h.snippet.clone(),
        )
    };
    hits.iter().map(one).collect()
}

/// 8 random queries, the same on both sides, the same hits; how many.
fn agree(
    rng: &mut Rng,
    split: &Side,
    one: &Side,
) -> Result<usize, Box<dyn StdError>> {
    let mut found = 0_usize;
    let (mut a, mut b) =
        (engine(&split.s, &split.cfg)?, engine(&one.s, &one.cfg)?);
    for _ in 0..8 {
        let state = rng.0;
        let qa = random_query(rng, split);
        rng.0 = state;
        let qb = random_query(rng, one);
        let (ha, hb) = (a.run(&qa, None)?, b.run(&qb, None)?);
        assert_eq!(key(&ha), key(&hb), "{qa:?}");
        found = found.saturating_add(ha.len());
    }
    Ok(found)
}

#[test]
fn merged_ranking_equals_1_unpartitioned_table_of_the_same_rows() -> Test {
    let mut found = 0_usize;
    for seed in 0..10 {
        let rng = &mut Rng(seed);
        let rows = fixture(rng);
        let (split, one) = sides(seed, &rows)?;
        assert!(parts(&split)? > 1, "seed {seed}");
        assert_eq!(parts(&one)?, 1, "seed {seed}");
        found = found.saturating_add(agree(rng, &split, &one)?);
    }
    assert!(found > 50, "only {found} hits");
    Ok(())
}
