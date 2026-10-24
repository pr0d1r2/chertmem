//! Search tests, and the scratch data dir every search test opens.

use std::error::Error as StdError;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle};

use serde_json::Value;

use super::{Engine, Hit, Query};
use crate::config::{Config, load_from};
use crate::store::{Event, Lookup, Mode, Ref, Store};

pub(super) type Test = Result<(), Box<dyn StdError>>;

pub(super) const DAY: u64 = 86_400_000;
/// 2026-01-01T00:00:00Z: inside 1 `120mo` bucket, 2020-01 to 2030-01.
pub(super) const BASE: u64 = 1_767_225_600_000;

/// A scratch dir holding `project/` and the data dir `data/`, removed on
/// drop.
pub(super) struct Scratch(PathBuf);

impl Scratch {
    pub(super) fn new(name: &str) -> Result<Self, Box<dyn StdError>> {
        let pid = std::process::id();
        let dir =
            std::env::temp_dir().join(format!("chert-search-{pid}-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("project"))?;
        Ok(Self(dir))
    }

    pub(super) fn root(&self) -> PathBuf {
        self.0.join("project")
    }

    pub(super) fn data(&self) -> PathBuf {
        self.0.join("data")
    }

    /// No user file, data in `data/`.
    pub(super) fn config(&self) -> Result<Config, Box<dyn StdError>> {
        config_at(&self.0)
    }

    pub(super) fn open(
        &self,
        cfg: &Config,
    ) -> Result<Store, Box<dyn StdError>> {
        Ok(Store::open(cfg, &self.root(), Mode::Read)?)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn config_at(base: &Path) -> Result<Config, Box<dyn StdError>> {
    let mut cfg = load_from(None, &base.join("project"))?.config;
    cfg.storage.data_dir = Some(base.join("data"));
    Ok(cfg)
}

pub(super) fn event(ts: u64, session: &str, body: &str) -> Event {
    Event {
        ts,
        session: session.into(),
        kind: "post-tool-use".into(),
        ide: "claude-code".into(),
        identity: None,
        body: body.into(),
    }
}

pub(super) fn engine(
    s: &Scratch,
    cfg: &Config,
) -> Result<Engine, Box<dyn StdError>> {
    Ok(Engine::open(cfg, &s.root())?)
}

pub(super) fn query(text: &str) -> Query {
    Query {
        text: text.into(),
        ..Query::default()
    }
}

/// The ULID behind each hit, through the store.
fn ids(
    store: &mut Store,
    hits: &[Hit],
) -> Result<Vec<String>, Box<dyn StdError>> {
    let refs: Vec<Ref> = hits.iter().map(|h| h.reference.clone()).collect();
    let found = store.get(&refs)?.into_iter().map(|l| match l {
        Lookup::Found(o) => Ok(o.id),
        Lookup::Dropped(r) => Err(format!("{r} dropped")),
    });
    Ok(found.collect::<Result<_, _>>()?)
}

/// A scratch dir, its config and a reading store on it.
pub(super) fn fresh(
    name: &str,
) -> Result<(Scratch, Config, Store), Box<dyn StdError>> {
    let s = Scratch::new(name)?;
    let cfg = s.config()?;
    let store = s.open(&cfg)?;
    Ok((s, cfg, store))
}

/// `cargo`, edited.
fn cargo(edit: impl FnOnce(&mut Query)) -> Query {
    let mut q = query("cargo");
    edit(&mut q);
    q
}

fn count(e: &mut Engine, q: &Query) -> Result<usize, Box<dyn StdError>> {
    Ok(e.run(q, None)?.len())
}

#[test]
fn a_text_without_terms_or_a_store_without_rows_finds_nothing() -> Test {
    let (s, cfg, mut store) = fresh("run-empty")?;
    let mut e = engine(&s, &cfg)?;
    assert!(e.run(&query("cargo"), None)?.is_empty());
    store.append(&event(BASE, "s", "cargo"))?;
    assert!(e.run(&query(" -- "), None)?.is_empty());
    assert_eq!(count(&mut e, &query("cargo"))?, 1);
    Ok(())
}

#[test]
fn a_hit_carries_ref_snippet_session_and_ts_best_first() -> Test {
    let (s, cfg, mut store) = fresh("run-hit")?;
    store.append(&event(BASE, "s1", "cargo build failed"))?;
    store.append(&event(BASE + 9 * DAY, "s2", "cargo cargo"))?;
    store.append(&event(BASE + 20 * DAY, "s3", "git push"))?;
    let hits = engine(&s, &cfg)?.run(&query("Cargo"), None)?;
    let row = |h: &Hit| (h.snippet.clone(), h.session.clone(), h.ts);
    let got: Vec<_> = hits.iter().map(row).collect();
    let late = ("cargo cargo".into(), "s2".into(), BASE + 9 * DAY);
    assert_eq!(
        got,
        [late, ("cargo build failed".into(), "s1".into(), BASE)]
    );
    assert!(hits.iter().all(|h| h.score > 0.0));
    assert_eq!(ids(&mut store, &hits)?.len(), 2);
    Ok(())
}

#[test]
fn limit_defaults_to_search_default_limit() -> Test {
    let (s, mut cfg, mut store) = fresh("run-limit")?;
    cfg.search.default_limit = 3;
    for ts in [
        BASE,
        BASE + DAY,
        BASE + 2 * DAY,
        BASE + 3 * DAY,
        BASE + 4 * DAY,
    ] {
        store.append(&event(ts, "s", "cargo"))?;
    }
    let mut e = engine(&s, &cfg)?;
    assert_eq!(count(&mut e, &query("cargo"))?, 3);
    assert_eq!(count(&mut e, &cargo(|q| q.limit = Some(4)))?, 4);
    Ok(())
}

/// 2 rows in this project (the later a `stop`), 1 in another.
fn two_projects(
    s: &Scratch,
    cfg: &Config,
) -> Result<String, Box<dyn StdError>> {
    let mut store = s.open(cfg)?;
    store.append(&event(BASE, "s1", "cargo"))?;
    let mut late = event(BASE + 10 * DAY, "s2", "cargo");
    late.kind = "stop".into();
    store.append(&late)?;
    let other = s.root().join("other");
    fs::create_dir_all(&other)?;
    let mut elsewhere = Store::open(cfg, &other, Mode::Read)?;
    elsewhere.append(&event(BASE, "s1", "cargo"))?;
    Ok(store.project().to_owned())
}

#[test]
fn project_since_kind_and_session_filter_rows() -> Test {
    let (s, cfg, _) = fresh("run-filter")?;
    let project = two_projects(&s, &cfg)?;
    let mut e = engine(&s, &cfg)?;
    assert_eq!(count(&mut e, &query("cargo"))?, 3);
    assert_eq!(count(&mut e, &cargo(|q| q.project = Some(project)))?, 2);
    assert_eq!(count(&mut e, &cargo(|q| q.since = Some(BASE + DAY)))?, 1);
    assert_eq!(count(&mut e, &cargo(|q| q.kind = Some("stop".into())))?, 1);
    let s1 = cargo(|q| q.session = Some("s1".into()));
    assert_eq!(count(&mut e, &s1)?, 2);
    Ok(())
}

#[test]
fn equal_scores_order_by_ts_desc_then_id_desc() -> Test {
    let (s, cfg, mut store) = fresh("run-ties")?;
    let (mid, late) = (BASE + 3 * DAY, BASE + 9 * DAY);
    for ts in [BASE, mid, mid, mid, late] {
        store.append(&event(ts, "s", "cargo"))?;
    }
    let mut e = engine(&s, &cfg)?;
    let hits = e.run(&query("cargo"), None)?;
    let ts: Vec<u64> = hits.iter().map(|h| h.ts).collect();
    assert_eq!(ts, [late, mid, mid, mid, BASE]);
    let got = ids(&mut store, &hits)?;
    let tied = got.get(1..4).ok_or("short")?;
    assert!(tied.windows(2).all(|w| w.first() > w.get(1)), "{tied:?}");
    assert_eq!(e.run(&query("cargo"), None)?, hits);
    Ok(())
}

/// What a fake server gives back for the texts it was sent.
pub(super) type Answer = fn(&[String]) -> String;

/// The fake server thread.
pub(super) type Server = JoinHandle<Option<Value>>;

/// A fake ollama on a free local port answering 1 request; the thread
/// gives back the request's JSON body.
pub(super) fn serve(
    answer: Answer,
) -> Result<(String, Server), Box<dyn StdError>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?.to_string();
    let worker = thread::spawn(move || {
        let (tcp, _) = listener.accept().ok()?;
        respond(tcp, answer)
    });
    Ok((addr, worker))
}

fn respond(mut tcp: TcpStream, answer: Answer) -> Option<Value> {
    let body = request_body(&mut BufReader::new(tcp.try_clone().ok()?))?;
    let json: Value = serde_json::from_str(&body).ok()?;
    let texts: Vec<String> = json
        .get("input")?
        .as_array()?
        .iter()
        .filter_map(|t| t.as_str().map(str::to_owned))
        .collect();
    let out = answer(&texts);
    let head =
        format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", out.len());
    tcp.write_all(format!("{head}{out}").as_bytes()).ok()?;
    Some(json)
}

fn request_body(r: &mut impl BufRead) -> Option<String> {
    let mut length = 0;
    loop {
        let mut line = String::new();
        r.read_line(&mut line).ok()?;
        let lower = line.to_ascii_lowercase();
        if let Some(n) = lower.strip_prefix("content-length:") {
            length = n.trim().parse().ok()?;
        }
        if line == "\r\n" {
            break;
        }
    }
    let mut body = vec![0; length];
    r.read_exact(&mut body).ok()?;
    String::from_utf8(body).ok()
}

/// 1 vector per text: its length and 1.
pub(super) fn lengths(texts: &[String]) -> String {
    let each: Vec<String> =
        texts.iter().map(|t| format!("[{}, 1]", t.len())).collect();
    format!("{{\"embeddings\": [{}]}}", each.join(", "))
}
