//! Search tests, and the scratch data dir every search test opens.

use std::error::Error as StdError;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle};

use serde_json::Value;

use super::{Asked, Engine, Hit, Query, token};
use crate::config::{Config, Grammar, load_from};
use crate::grammar::{Intensity, compress};
use crate::hook::{self, Ctx, Ide, Kind};
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

/// `prompt` stored the way the hook stores it, at `cfg`'s grammar.
fn prompted(s: &Scratch, cfg: &Config, prompt: &str) -> Test {
    let stdin = serde_json::json!({
        "session_id": "s1",
        "hook_event_name": "UserPromptSubmit",
        "prompt": prompt,
    });
    let root = s.root();
    let ctx = Ctx {
        config: cfg,
        root: &root,
        now: BASE,
        ide: Ide::ClaudeCode,
    };
    let out =
        hook::run(Kind::UserPromptSubmit, stdin.to_string().as_bytes(), &ctx);
    out.note.map_or(Ok(()), |n| Err(n.into()))
}

/// The snippets `text` finds at `grammar`, after the hook stored
/// [`PROMPT`].
fn found_at(
    name: &str,
    grammar: Grammar,
    text: &str,
) -> Result<Vec<String>, Box<dyn StdError>> {
    found_in(name, grammar, PROMPT, text)
}

/// The snippets `text` finds at `grammar`, after the hook stored
/// `prompt`.
fn found_in(
    name: &str,
    grammar: Grammar,
    prompt: &str,
    text: &str,
) -> Result<Vec<String>, Box<dyn StdError>> {
    let (s, mut cfg, _) = fresh(name)?;
    cfg.compression.grammar = grammar;
    prompted(&s, &cfg, prompt)?;
    let hits = engine(&s, &cfg)?.run(&query(text), None)?;
    Ok(hits.into_iter().map(|h| h.snippet).collect())
}

const PROMPT: &str = "fix the middleware now";

#[test]
fn a_plain_word_finds_a_body_the_hook_stored_compressed() -> Test {
    let stored = compress(PROMPT, Intensity::Full);
    assert!(stored.contains("mw") && !stored.contains("middleware"));
    let got = found_at("v5-full", Grammar::Full, "middleware")?;
    assert_eq!(got, std::slice::from_ref(&stored));
    assert_eq!(found_at("v5-short", Grammar::Full, "mw")?, [stored]);
    Ok(())
}

#[test]
fn with_grammar_off_plain_text_is_stored_and_found() -> Test {
    let got = found_at("v5-off", Grammar::Off, "middleware")?;
    assert_eq!(got, [PROMPT]);
    Ok(())
}

#[test]
fn a_query_of_only_words_the_grammar_drops_finds_nothing() -> Test {
    let (s, cfg, mut store) = fresh("v5-dropped")?;
    store.append(&event(BASE, "s", "the a cargo"))?;
    let mut e = engine(&s, &cfg)?;
    assert!(e.run(&query("the a"), None)?.is_empty());
    assert_eq!(count(&mut e, &query("the cargo"))?, 1);
    Ok(())
}

/// The terms `text` gives at `full`, as a query or a stored body.
fn full(text: &str) -> Vec<String> {
    token::terms(&compress(text, Intensity::Full))
}

#[test]
fn a_lone_word_compresses_as_it_does_inside_prose() {
    let prose = [
        ("middleware", "we saw the middleware fail"),
        ("Database", "The database is down."),
        ("dependencies", "update dependencies, then"),
    ];
    for (word, sentence) in prose {
        let body = full(sentence);
        let lone = full(word);
        assert!(lone.iter().all(|t| body.contains(t)), "{word}: {body:?}");
    }
    assert_eq!(full("middleware"), ["mw"]);
}

#[test]
fn kept_spans_and_dropped_phrases_differ_from_a_lone_word() {
    let kept = [
        "see middleware_x now",
        "see `middleware` now",
        "see src/middleware.rs now",
        "a middleware-based y",
    ];
    let (mw, long) = ("mw".to_owned(), "middleware".to_owned());
    for sentence in kept {
        let body = full(sentence);
        assert!(body.contains(&long) && !body.contains(&mw), "{body:?}");
    }
    assert_eq!(full("kind"), ["kind"]);
    assert_eq!(full("the kind of x"), ["x"]);
    assert_eq!(full("think"), ["think"]);
    assert!(full("i think that").is_empty());
}

#[test]
fn a_plain_word_finds_its_long_form_kept_in_a_path_or_code() -> Test {
    for (name, prompt) in [
        ("t7-path", "see src/middleware.rs now"),
        ("t7-code", "see `middleware` now"),
        ("t7-compound", "a middleware-based y"),
    ] {
        let stored = compress(prompt, Intensity::Full);
        assert!(stored.contains("middleware"), "{stored}");
        let got = found_in(name, Grammar::Full, prompt, "middleware")?;
        assert_eq!(got, [stored]);
    }
    Ok(())
}

#[test]
fn the_plain_and_the_compressed_form_each_score_once() -> Test {
    let (s, mut cfg, mut store) = fresh("t7-both")?;
    cfg.compression.grammar = Grammar::Full;
    store.append(&event(BASE, "s", "mw middleware"))?;
    store.append(&event(BASE + DAY, "s", "mw"))?;
    store.append(&event(BASE + 2 * DAY, "s", "middleware"))?;
    let hits = engine(&s, &cfg)?.run(&query("middleware Middleware"), None)?;
    let ts: Vec<u64> = hits.iter().map(|h| h.ts).collect();
    assert_eq!(ts, [BASE, BASE + 2 * DAY, BASE + DAY]);
    let at = |i: usize| hits.get(i).map_or(0.0, |h| h.score);
    assert!(at(0) > at(1), "both forms outscore 1");
    assert!((at(1) - at(2)).abs() < 1e-12, "1 form, same df");
    Ok(())
}

#[test]
fn a_term_alike_in_both_forms_scores_as_at_grammar_off() -> Test {
    let (s, mut cfg, mut store) = fresh("t7-alike")?;
    store.append(&event(BASE, "s", "cargo build"))?;
    store.append(&event(BASE, "s", "rust run"))?;
    let mut scores = Vec::new();
    for grammar in [Grammar::Off, Grammar::Full] {
        cfg.compression.grammar = grammar;
        let hits = engine(&s, &cfg)?.run(&query("cargo"), None)?;
        scores.push(hits.iter().map(|h| h.score).collect::<Vec<_>>());
    }
    assert_eq!(scores.first(), scores.get(1));
    assert_eq!(scores.first().map(Vec::len), Some(1));
    Ok(())
}

#[test]
fn asked_holds_both_forms_and_embeds_the_raw_text_only_when_dropped() {
    let text = "fix the middleware now";
    let mw = Asked::new(text, Grammar::Full);
    assert_eq!(mw.text, compress(text, Intensity::Full));
    assert_eq!(mw.terms, ["fix", "middleware", "mw", "now"]);
    let dropped = Asked::new("i think that", Grammar::Full);
    assert_eq!(dropped.text, "i think that");
    assert_eq!(dropped.terms, ["i", "that", "think"]);
    assert!(Asked::new("the a", Grammar::Full).terms.is_empty());
}
