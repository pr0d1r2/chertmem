use std::error::Error as StdError;
use std::net::TcpListener;

use serde_json::Value;

use super::{Embed, blend, cosine, rerank};
use crate::config::{Config, Grammar, Provider};
use crate::grammar::{Intensity, compress};
use crate::search::rank::Scored;
use crate::search::tests::{
    BASE, DAY, Scratch, Test, event, fresh, lengths, query, serve,
};
use crate::search::{Engine, Error, Hit, Query};
use crate::store::Observation;

/// Gives back the same vectors whatever it is sent.
struct Fixed(Vec<Vec<f64>>);

impl Embed for Fixed {
    fn embed(&self, _: &[&str]) -> Result<Vec<Vec<f64>>, Error> {
        Ok(self.0.clone())
    }
}

fn scored(score: f64, ts: u64) -> Result<Scored, Box<dyn StdError>> {
    let row = Observation {
        reference: crate::store::Ref::parse("chert:0000000A")?,
        id: format!("{ts:08}"),
        project: "p".into(),
        ts,
        session: "s".into(),
        ide: "i".into(),
        kind: "k".into(),
        identity: None,
        body: "b".into(),
    };
    Ok(Scored { score, row })
}

#[test]
fn blend_weighs_keyword_and_vector_by_alpha() {
    assert!((blend(0.4, 0.8, 0.0) - 0.4).abs() < 1e-12);
    assert!((blend(0.4, 0.8, 1.0) - 0.8).abs() < 1e-12);
    assert!((blend(0.4, 0.8, 0.5) - 0.6).abs() < 1e-12);
}

#[test]
fn cosine_is_1_for_parallel_and_0_for_zero_or_mismatched() {
    assert!((cosine(&[1.0, 2.0], &[2.0, 4.0]) - 1.0).abs() < 1e-12);
    assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-12);
    assert!(cosine(&[0.0, 0.0], &[1.0, 1.0]).abs() < f64::EPSILON);
    assert!(cosine(&[1.0], &[1.0, 1.0]).abs() < f64::EPSILON);
}

#[test]
fn rerank_reorders_by_the_blend_and_refuses_a_short_answer() -> Test {
    let rows = vec![scored(2.0, 1)?, scored(1.0, 2)?];
    let near = Fixed(vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 0.0]]);
    let got = rerank(&near, "q", rows.clone(), 1.0)?;
    assert_eq!(got.iter().map(|s| s.row.ts).collect::<Vec<_>>(), [2, 1]);
    let kept = rerank(&near, "q", rows.clone(), 0.0)?;
    assert_eq!(kept.first().map(|s| s.score), Some(1.0));
    assert!(rerank(&Fixed(vec![vec![1.0]]), "q", rows, 0.5).is_err());
    assert_eq!(rerank(&near, "q", Vec::new(), 0.5)?, []);
    Ok(())
}

/// An engine whose `OLLAMA_HOST` is `host`, under `provider`.
fn engine_at(
    s: &Scratch,
    provider: Provider,
    host: &str,
) -> Result<(Engine, Config), Box<dyn StdError>> {
    let mut cfg = s.config()?;
    cfg.embedding.provider = provider;
    cfg.search.alpha = 1.0;
    Ok((Engine::with_host(&cfg, &s.root(), host.into())?, cfg))
}

fn semantic(text: &str) -> Query {
    Query {
        semantic: true,
        ..query(text)
    }
}

#[test]
fn not_semantic_or_provider_off_opens_no_connection() -> Test {
    let (s, _, mut store) = fresh("blend-quiet")?;
    store.append(&event(BASE, "s", "cargo"))?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let host = listener.local_addr()?.to_string();
    let (mut on, _) = engine_at(&s, Provider::Ollama, &host)?;
    assert_eq!(on.run(&query("cargo"), None)?.len(), 1);
    let (mut off, _) = engine_at(&s, Provider::Off, &host)?;
    assert_eq!(off.run(&semantic("cargo"), None)?.len(), 1);
    let quiet = listener.accept().map(|_| ()).map_err(|e| e.kind());
    assert_eq!(quiet, Err(std::io::ErrorKind::WouldBlock));
    Ok(())
}

fn order(hits: &[Hit]) -> Vec<u64> {
    hits.iter().map(|h| h.ts).collect()
}

#[test]
fn ollama_and_semantic_re_rank_by_the_vectors() -> Test {
    let (s, _, mut store) = fresh("blend-ollama")?;
    store.append(&event(BASE, "s", "cargo cargo cargo zz"))?;
    store.append(&event(BASE + DAY, "s", "cargo"))?;
    let (addr, server) = serve(lengths)?;
    let (mut e, _) = engine_at(&s, Provider::Ollama, &addr)?;
    assert_eq!(order(&e.run(&query("cargo"), None)?), [BASE, BASE + DAY]);
    assert_eq!(order(&e.run(&semantic("cargo"), None)?), [BASE + DAY, BASE]);
    assert!(server.join().map_err(|_| "server panicked")?.is_some());
    Ok(())
}

#[test]
fn the_re_rank_embeds_the_compressed_query_it_scores() -> Test {
    let (s, _, mut store) = fresh("blend-text")?;
    store.append(&event(BASE, "s", "fix mw now"))?;
    let (addr, server) = serve(lengths)?;
    let (mut e, _) = engine_at(&s, Provider::Ollama, &addr)?;
    e.cfg.compression.grammar = Grammar::Full;
    let text = "fix the middleware now";
    assert_eq!(e.run(&semantic(text), None)?.len(), 1);
    let sent = server.join().map_err(|_| "server panicked")?;
    let first = sent.as_ref().and_then(|j| j.get("input")?.get(0).cloned());
    let want = compress(text, Intensity::Full);
    assert_eq!(first.as_ref().and_then(Value::as_str), Some(want.as_str()));
    assert_ne!(want, text);
    Ok(())
}

#[test]
fn openai_falls_back_to_keyword_ranking() -> Test {
    let (s, _, mut store) = fresh("blend-openai")?;
    store.append(&event(BASE, "s", "cargo"))?;
    let (mut e, _) = engine_at(&s, Provider::Openai, "")?;
    let keyword = e.run(&query("cargo"), None)?;
    assert_eq!(keyword.len(), 1);
    assert_eq!(e.run(&semantic("cargo"), None)?, keyword);
    assert_eq!(e.run(&semantic("cargo"), None)?, keyword);
    Ok(())
}

#[test]
fn open_reads_ollama_host_only_for_ollama() -> Test {
    let s = Scratch::new("blend-open")?;
    let mut cfg = s.config()?;
    cfg.embedding.provider = Provider::Off;
    assert_eq!(Engine::open(&cfg, &s.root())?.host, "");
    cfg.embedding.provider = Provider::Ollama;
    let want = std::env::var("OLLAMA_HOST").unwrap_or_default();
    assert_eq!(Engine::open(&cfg, &s.root())?.host, want);
    Ok(())
}
