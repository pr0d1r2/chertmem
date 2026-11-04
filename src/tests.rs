//! V1: the module graph, read from `crate::` paths, is the edge list
//! `src/SPEC.md` states, and has no cycle.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

type Test = Result<(), Box<dyn Error>>;
type Graph = BTreeMap<String, BTreeSet<String>>;

const MODULES: [&str; 11] = [
    "cache", "cli", "config", "grammar", "hook", "import", "install", "mcp",
    "mine", "search", "store",
];

/// The module that is every other's caller, so no list of its own.
const TOP: &str = "cli";

fn src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The first identifier of `text`.
fn ident(text: &str) -> &str {
    let text = text.trim_start();
    text.split(|c| !is_ident(c)).next().unwrap_or_default()
}

/// The `{...}` group `group` opens, up to its closing brace, with every
/// nested group cut out: `a::{b}, c}` -> `a::, c`.
fn top_level(group: &str) -> String {
    let mut depth = 0_usize;
    let mut kept = String::new();
    for c in group.chars() {
        match c {
            '{' => depth = depth.saturating_add(1),
            '}' if depth == 0 => break,
            '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => kept.push(c),
            _ => {}
        }
    }
    kept
}

/// The names 1 `crate::` path reaches, `rest` being what follows it.
fn reached(rest: &str) -> Vec<String> {
    match rest.strip_prefix('{') {
        Some(group) => top_level(group)
            .split(',')
            .map(|item| ident(item).to_owned())
            .filter(|name| !name.is_empty())
            .collect(),
        None => vec![ident(rest).to_owned()],
    }
}

/// The module names `code` reaches through `crate::`, groups across
/// lines included.
fn paths_in(code: &str) -> Vec<String> {
    code.match_indices("crate::")
        .filter_map(|(at, word)| code.get(at.checked_add(word.len())?..))
        .flat_map(reached)
        .collect()
}

/// `text` with its comments and string literals cut out, each left as
/// 1 space, so an inline comment inside a group hides no member and a
/// string names no module.
fn code(text: &str) -> String {
    let mut kept = String::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        let skip = skipped(rest);
        let step = if skip > 0 { skip } else { c.len_utf8() };
        kept.push(if skip > 0 { ' ' } else { c });
        rest = rest.get(step..).unwrap_or_default();
    }
    kept
}

/// The bytes of the comment or literal `rest` opens; 0 when it opens
/// none. A line comment stops before its newline.
fn skipped(rest: &str) -> usize {
    if rest.starts_with("//") {
        return rest.find('\n').unwrap_or(rest.len());
    }
    if rest.starts_with("/*") {
        return block(rest);
    }
    match rest.chars().next() {
        Some('"') => quoted(rest),
        Some('\'') => char_literal(rest),
        Some('r') => raw(rest),
        _ => 0,
    }
}

/// The bytes of the block comment `rest` opens, nested ones included.
fn block(rest: &str) -> usize {
    let mut depth = 0_usize;
    let mut at = 0_usize;
    while let Some(tail) = rest.get(at..).filter(|t| !t.is_empty()) {
        let step = match tail.get(..2) {
            Some("/*") => (depth.saturating_add(1), 2),
            Some("*/") => (depth.saturating_sub(1), 2),
            _ => (depth, tail.chars().next().map_or(1, char::len_utf8)),
        };
        let closed = depth == 1 && step.0 == 0;
        (depth, at) = (step.0, at.saturating_add(step.1));
        if closed {
            return at;
        }
    }
    rest.len()
}

/// The bytes of the string literal `rest` opens, escapes skipped.
fn quoted(rest: &str) -> usize {
    let mut chars = rest.char_indices().skip(1);
    while let Some((at, c)) = chars.next() {
        match c {
            '\\' => _ = chars.next(),
            '"' => return at.saturating_add(1),
            _ => {}
        }
    }
    rest.len()
}

/// The bytes of the char literal `rest` opens; 0 for a lifetime.
fn char_literal(rest: &str) -> usize {
    let mut chars = rest.char_indices().skip(1);
    let end = match chars.next() {
        Some((_, '\\')) => chars.skip(1).find(|&(_, c)| c == '\''),
        Some(_) => chars.next().filter(|&(_, c)| c == '\''),
        None => None,
    };
    end.map_or(0, |(at, _)| at.saturating_add(1))
}

/// The bytes of the raw string `rest` opens: `r"..."`, `r#"..."#`; 0
/// when `r` opens none, as in `r#type` or a name.
fn raw(rest: &str) -> usize {
    let tail = rest.get(1..).unwrap_or_default();
    let body = tail.trim_start_matches('#');
    let hashes = tail.len().saturating_sub(body.len());
    let Some(inner) = body.strip_prefix('"') else {
        return 0;
    };
    let close = format!("\"{}", "#".repeat(hashes));
    let open = rest.len().saturating_sub(inner.len());
    inner.find(&close).map_or(rest.len(), |at| {
        open.saturating_add(at).saturating_add(close.len())
    })
}

/// Every non-test `.rs` of `dir`, deep.
fn sources(dir: &Path, found: &mut Vec<PathBuf>) -> Test {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path.file_name().and_then(|n| n.to_str());
        if name == Some("tests.rs") || name == Some("tests") {
            continue;
        }
        if path.is_dir() {
            sources(&path, found)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
    Ok(())
}

/// The modules `module`'s code names, itself left out.
fn uses(module: &str) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let mut files = vec![src().join(format!("{module}.rs"))];
    sources(&src().join(module), &mut files)?;
    let mut names = BTreeSet::new();
    for file in files {
        names.extend(paths_in(&code(&fs::read_to_string(file)?)));
    }
    names.retain(|n| n != module && MODULES.contains(&n.as_str()));
    Ok(names)
}

fn built() -> Result<Graph, Box<dyn Error>> {
    let mut graph = Graph::new();
    for module in MODULES {
        graph.insert(module.to_owned(), uses(module)?);
    }
    Ok(graph)
}

/// 1 clause of the list: `` `a`, `b` → c, d (why) ``, or `` `a` leaf ``.
fn clause(text: &str, graph: &mut Graph) {
    let (from, to) = text.split_once('→').unwrap_or((text, ""));
    let to = to.split(" (").next().unwrap_or_default();
    let to: BTreeSet<String> = to
        .split(',')
        .map(|n| ident(n).to_owned())
        .filter(|n| !n.is_empty())
        .collect();
    for name in from.split('`').skip(1).step_by(2) {
        graph.insert(name.to_owned(), to.clone());
    }
}

/// The edge list of `src/SPEC.md` C.
fn stated() -> Result<Graph, Box<dyn Error>> {
    let spec = fs::read_to_string(src().join("SPEC.md"))?;
    let mut lines = spec.lines();
    let line = lines
        .find(|l| l.starts_with("- module edges acyclic"))
        .ok_or("no edge list")?;
    let (_, list) = line.split_once("): ").ok_or("no list")?;
    let mut graph = Graph::new();
    list.trim_end_matches('.')
        .split(';')
        .for_each(|c| clause(c, &mut graph));
    Ok(graph)
}

fn acyclic(graph: &Graph) -> bool {
    let mut left = graph.clone();
    while let Some(free) = left
        .iter()
        .find(|(_, to)| to.iter().all(|t| !left.contains_key(t)))
        .map(|(name, _)| name.clone())
    {
        left.remove(&free);
    }
    left.is_empty()
}

#[test]
fn the_built_graph_is_the_stated_graph() -> Test {
    let mut built = built()?;
    let mut stated = stated()?;
    built.remove(TOP);
    stated.remove(TOP);
    assert_eq!(built, stated);
    Ok(())
}

#[test]
fn the_built_graph_has_no_cycle_and_nothing_uses_the_top() -> Test {
    let graph = built()?;
    assert!(acyclic(&graph), "{graph:?}");
    assert!(!graph.values().any(|to| to.contains(TOP)), "{graph:?}");
    Ok(())
}

#[test]
fn nested_and_multi_line_groups_are_read_whole() {
    let code =
        "use crate::{\n    cache::{Cache, Handle},\n    store::Store,\n};";
    assert_eq!(paths_in(code), ["cache", "store"]);
    assert_eq!(paths_in("use crate::{a, b};"), ["a", "b"]);
    assert_eq!(paths_in("let x = crate::grammar::expand(y);"), ["grammar"]);
}

#[test]
fn comments_inside_a_group_hide_no_member_and_strings_name_none() {
    let line = "use crate::{grammar::Grammar, // parser\n    store::Store};";
    assert_eq!(paths_in(&code(line)), ["grammar", "store"]);
    let block = "use crate::{/* a, */ cache, /* b */ mine};";
    assert_eq!(paths_in(&code(block)), ["cache", "mine"]);
    let text = "let s = \"// crate::hook\"; use crate::store; // crate::cli";
    assert_eq!(paths_in(&code(text)), ["store"]);
    let raw = "let q = '\"'; let r = r#\"a \"crate::hook\" b\"#; crate::mine";
    assert_eq!(paths_in(&code(raw)), ["mine"]);
    let nested =
        "/* a /* crate::cli */ crate::hook */ fn f<'a>() { crate::mcp }";
    assert_eq!(paths_in(&code(nested)), ["mcp"]);
}

#[test]
fn a_cycle_is_seen() {
    let edge = |to: &str| BTreeSet::from([to.to_owned()]);
    let graph =
        Graph::from([("a".to_owned(), edge("b")), ("b".to_owned(), edge("a"))]);
    assert!(!acyclic(&graph));
}
