use std::error::Error as StdError;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::{Job, Pool, candidates, parallel, split, workers};
use crate::config::{Config, Dimension, Span, Unit};
use crate::search::rank::{Rater, top};
use crate::search::terms::{Stats, Terms};
use crate::search::tests::{
    BASE, DAY, Scratch, Test, engine, event, fresh, query,
};
use crate::search::{Engine, Hit, Query};
use crate::store::{Filter, Mode, Store, project_id};

fn paths(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("p{i}")).collect()
}

/// Stats before any row is counted, for 1 term.
fn empty() -> Stats {
    Stats {
        docs: 0,
        tokens: 0,
        df: vec![0],
    }
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
    let (filter, root, pool) =
        (Filter::default(), s.root(), &mut Pool::default());
    let got = parallel(&job(&cfg, &root, &filter, &rater), pool, &paths)?;
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
        generation: 0,
        filter,
        rater,
    }
}

#[test]
fn the_pool_keeps_its_stores_until_the_generation_moves() -> Test {
    let (s, cfg, _) = fresh("merge-pool")?;
    let (filter, root) = (Filter::default(), s.root());
    let rater = rater(empty(), 1);
    let mut j = job(&cfg, &root, &filter, &rater);
    let mut pool = Pool::default();
    pool.fill(&j, 2)?;
    pool.fill(&j, 1)?;
    assert_eq!((pool.generation, pool.stores.len()), (0, 2));
    j.generation = 7;
    pool.fill(&j, 1)?;
    assert_eq!((pool.generation, pool.stores.len()), (7, 1));
    Ok(())
}

#[test]
fn a_query_after_a_drop_misses_only_the_dropped_rows() -> Test {
    let (s, cfg, mut store) = fresh("merge-drop")?;
    let paths = twelve(&mut store)?;
    let mut e = engine(&s, &cfg)?;
    let mut q = query("cargo");
    q.limit = Some(20);
    assert_eq!(e.run(&q, None)?.len(), 12);
    store.drop(paths.first().ok_or("none")?)?;
    assert_eq!(e.run(&q, None)?.len(), 11);
    Ok(())
}

#[test]
fn a_worker_that_cannot_open_its_store_fails_the_query() -> Test {
    let s = Scratch::new("merge-fail")?;
    let mut cfg = s.config()?;
    cfg.storage.data_dir = None;
    let (filter, root, rater) =
        (Filter::default(), s.root(), rater(empty(), 1));
    let found = parallel(
        &job(&cfg, &root, &filter, &rater),
        &mut Pool::default(),
        &paths(2),
    );
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

#[test]
fn a_partition_gives_only_the_rows_its_index_matches() -> Test {
    let (s, cfg, mut store) = fresh("merge-fts")?;
    for (ts, body) in [(BASE, "cargo build"), (BASE + 1, "git push")] {
        store.append(&event(ts, "s", body))?;
    }
    store.append(&event(BASE + 2, "s", "Cargo, test"))?;
    let parts = store.partitions(&Filter::default())?;
    let path = parts.first().map(|p| p.path.clone()).ok_or("none")?;
    let (filter, root, rater) =
        (Filter::default(), s.root(), rater(empty(), 9));
    let job = job(&cfg, &root, &filter, &rater);
    let got = candidates(&mut store, &path, &job)?;
    let bodies: Vec<&str> = got.iter().map(|o| o.body.as_str()).collect();
    assert_eq!(bodies, ["cargo build", "Cargo, test"]);
    Ok(())
}

// The T6 benchmark (src/search T6, root C: search < 10 ms): 10^5 rows
// over about 10 weekly partitions, 10 words each from 1000, a 2-term
// query, the median of warm runs from disk. Ignored by default; run it
// with `cargo nextest run --release --run-ignored only -E
// 'test(bench)' --no-capture`.

const ROWS: u64 = 100_000;
/// 1 row a minute: 10^5 minutes are about 10 weeks.
const MINUTE: u64 = 60_000;
const WORDS: u64 = 1000;
const RUNS: usize = 21;
const MEDIAN: usize = 10;
const TARGET: Duration = Duration::from_millis(10);

fn body(rng: &mut Rng) -> String {
    let words: Vec<String> =
        (0..10).map(|_| format!("w{}", rng.below(WORDS))).collect();
    words.join(" ")
}

/// The median of [`RUNS`] runs of `q`.
fn median(e: &mut Engine, q: &Query) -> Result<Duration, Box<dyn StdError>> {
    let mut took = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let start = Instant::now();
        e.run(q, None)?;
        took.push(start.elapsed());
    }
    took.sort_unstable();
    Ok(took.get(MEDIAN).copied().unwrap_or(Duration::MAX))
}

#[test]
#[ignore = "benchmark: appends 10^5 rows; run it in release"]
fn bench_a_2_term_query_over_10_5_rows_takes_under_10_ms() -> Test {
    let (s, cfg, mut store) = fresh("bench-t6")?;
    let rng = &mut Rng(6);
    for i in 0..ROWS {
        let ts = BASE.saturating_add(i.saturating_mul(MINUTE));
        store.append(&event(ts, "s", &body(rng)))?;
    }
    let mut e = engine(&s, &cfg)?;
    let q = query("w1 w2");
    let found = e.run(&q, None)?.len();
    let median = median(&mut e, &q)?;
    eprintln!("src/search T6: {ROWS} rows, 2 terms, median {median:?}");
    assert!(found > 0 && median < TARGET, "{found} hits, {median:?}");
    Ok(())
}

// V6 (B2): a search over 520 partitions holds a bounded number of open
// files, so it works under macOS's default limit of 256. `ulimit`
// cannot be set without `unsafe`, so the test counts this process's
// open descriptors instead.

/// The descriptors this process holds open, as its fd dir lists them.
fn open_fds() -> Result<usize, Box<dyn StdError>> {
    let dir = if cfg!(target_os = "linux") {
        "/proc/self/fd"
    } else {
        "/dev/fd"
    };
    Ok(fs::read_dir(dir)?.count())
}

/// 3 descriptors a partition connection (file, `-wal`, `-shm`) for at
/// most 64 connections, and slack for the catalog and terms files.
const FD_BOUND: usize = 3 * 64 + 32;

#[test]
fn a_search_over_520_partitions_holds_a_bounded_number_of_files() -> Test {
    let (s, cfg, mut store) = fresh("merge-fds")?;
    for i in 0..520_u64 {
        let ts = BASE.saturating_add(i.saturating_mul(8 * DAY));
        store.append(&event(ts, "s", "cargo test"))?;
    }
    drop(store);
    let before = open_fds()?;
    let mut e = engine(&s, &cfg)?;
    let found = e.run(&query("cargo"), None)?.len();
    let held = open_fds()?.saturating_sub(before);
    eprintln!("V6: {before} fds before, {held} more after a search");
    assert!(found > 0 && held <= FD_BOUND, "{found} hits, {held} fds");
    Ok(())
}
