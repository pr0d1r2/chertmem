use std::collections::BTreeMap;
use std::error::Error as StdError;
use std::mem::size_of;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::{Posting, Postings};
use crate::cache::{Cache, Env, Handle, Index, Key};
use crate::search::rank::Rater;
use crate::search::terms::Stats;
use crate::search::tests::{BASE, DAY, Test, engine, event, fresh, query};
use crate::search::{Engine, Query, index};
use crate::store::{Observation, Ref};

type Res<T> = Result<T, Box<dyn StdError>>;

/// What an index holds, slots aside: per term the rows and their tf,
/// per row its token count.
type Canon = (BTreeMap<String, BTreeMap<Key, u32>>, BTreeMap<Key, u32>);

/// The rows `list` posts, each once.
fn posted(p: &Postings, list: &[Posting]) -> Res<BTreeMap<Key, u32>> {
    let key = |slot: u32| -> Res<Key> {
        let at = usize::try_from(slot)?;
        Ok(p.keys.get(at).ok_or("a slot past the keys")?.clone())
    };
    let rows: Res<BTreeMap<Key, u32>> = list
        .iter()
        .map(|post| Ok((key(post.slot)?, post.tf)))
        .collect();
    let rows = rows?;
    assert_eq!(rows.len(), list.len(), "a row posted twice");
    Ok(rows)
}

fn canon(p: &Postings) -> Res<Canon> {
    let mut terms = BTreeMap::new();
    for (term, list) in &p.terms {
        terms.insert(term.clone(), posted(p, list)?);
    }
    let lengths = p.keys.iter().cloned().zip(p.lengths.iter().copied());
    Ok((terms, lengths.collect()))
}

fn row(n: u64, body: &str) -> Res<Observation> {
    let id = format!("{n:08X}");
    Ok(Observation {
        reference: Ref::parse(&format!("chert:{id}"))?,
        id,
        project: "p".into(),
        ts: n,
        session: "s".into(),
        ide: "claude-code".into(),
        kind: "post-tool-use".into(),
        identity: None,
        body: body.into(),
    })
}

/// `SplitMix64`: the same ops on every run.
fn next(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *seed;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

const WORDS: [&str; 6] = ["cargo", "test", "git", "mw", "middleware", "a"];

/// A body of 0 to 5 words, repeats likely.
fn body(seed: &mut u64) -> String {
    let n = next(seed) % 6;
    let pick = |s: &mut u64| {
        let at = usize::try_from(next(s) % 6).unwrap_or_default();
        WORDS.get(at).copied().unwrap_or_default()
    };
    let words: Vec<&str> = (0..n).map(|_| pick(seed)).collect();
    words.join(" ")
}

/// `ops` inserts over 12 keys, so most replace a row; the rows kept.
fn feed(
    p: &mut Postings,
    seed: &mut u64,
    ops: u32,
) -> Res<BTreeMap<Key, Observation>> {
    let mut rows = BTreeMap::new();
    for _ in 0..ops {
        let o = row(next(seed) % 12, &body(seed))?;
        let key = (o.ts, o.id.clone());
        p.insert(&key, &o, rows.get(&key));
        rows.insert(key, o);
    }
    Ok(rows)
}

fn rebuilt(rows: &BTreeMap<Key, Observation>) -> Postings {
    let mut p = Postings::default();
    for (key, o) in rows {
        p.insert(key, o, None);
    }
    p
}

#[test]
fn inserts_replaces_and_a_reset_equal_a_rebuild_of_the_final_rows() -> Test {
    for start in 0..20 {
        let (mut seed, mut p) = (start, Postings::default());
        feed(&mut p, &mut seed, 60)?;
        p.reset();
        let rows = feed(&mut p, &mut seed, 80)?;
        assert_eq!(canon(&p)?, canon(&rebuilt(&rows))?, "seed {start}");
    }
    Ok(())
}

fn rater(terms: &[&str]) -> Rater {
    Rater {
        stats: Stats {
            docs: 40,
            tokens: 90,
            df: vec![7; terms.len()],
        },
        terms: terms.iter().map(|&t| t.to_owned()).collect(),
        kind: None,
        session: None,
        keep: 5,
    }
}

#[test]
fn indexed_counts_score_the_bits_whole_bodies_do() -> Test {
    for start in 0..20 {
        let (mut seed, mut p) = (start, Postings::default());
        let rows = feed(&mut p, &mut seed, 40)?;
        let r = rater(&["a", "cargo", "mw", "zzz"]);
        let hits = p.hits(&r.terms).ok_or("no answer")?;
        let lent = hits
            .into_iter()
            .filter_map(|(k, c)| Some((rows.get(k)?, c)));
        let whole = r.best(rows.values().cloned().collect());
        assert_eq!(r.best_counted(lent), whole, "seed {start}");
    }
    Ok(())
}

#[test]
fn bytes_count_capacity_and_a_reset_frees_them() -> Res<()> {
    let mut p = Postings::default();
    assert_eq!(p.bytes(), 0);
    feed(&mut p, &mut 7, 30)?;
    let floor = p.keys.capacity() * size_of::<Key>() + p.terms.capacity();
    assert!(p.bytes() > u64::try_from(floor)?, "{}", p.bytes());
    p.reset();
    assert_eq!(p.bytes(), 0);
    Ok(())
}

#[test]
fn a_full_index_answers_nothing() -> Res<()> {
    let mut p = Postings::default();
    feed(&mut p, &mut 3, 5)?;
    p.full = true;
    assert!(p.hits(&["a".to_owned()]).is_none());
    Ok(())
}

fn env() -> Env {
    Env {
        now: || BASE + 30 * DAY,
        memory: || None,
        log: Arc::new(|_: &str| {}),
    }
}

fn warm(handle: &Handle) -> Test {
    for _ in 0..500 {
        if handle.is_warm()? {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(10));
    }
    Err("the cache never warmed".into())
}

#[test]
fn an_indexed_cache_ranks_as_the_disk_does() -> Test {
    let (s, cfg, mut store) = fresh("postings-v1")?;
    store.append(&event(BASE + 5 * DAY, "s", "cargo build"))?;
    store.append(&event(BASE + 6 * DAY, "t", "cargo cargo test"))?;
    store.append(&event(BASE + 12 * DAY, "s", "git push cargo"))?;
    let handle = Cache::start_indexed(s.open(&cfg)?, &cfg, env(), index());
    warm(&handle)?;
    let mut e = engine(&s, &cfg)?;
    let mut q = query("cargo test");
    (q.project, q.since) = (Some(e.project().into()), Some(BASE + 6 * DAY));
    same(&mut e, &q, &handle, 2)?;
    store.append(&event(BASE + 20 * DAY, "s", "test test"))?;
    q.session = Some("s".into());
    same(&mut e, &q, &handle, 2)
}

/// `q` finds `n` hits from disk and the same, bit for bit, from `cache`.
fn same(e: &mut Engine, q: &Query, cache: &Handle, n: usize) -> Test {
    let disk = e.run(q, None)?;
    assert_eq!((disk.len(), e.run(q, Some(cache))?), (n, disk.clone()));
    Ok(())
}
