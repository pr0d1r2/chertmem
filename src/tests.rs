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

/// The names 1 `crate::` path reaches, `rest` being what follows it.
fn reached(rest: &str) -> Vec<String> {
    match rest.strip_prefix('{') {
        Some(group) => group
            .split('}')
            .next()
            .unwrap_or_default()
            .split(',')
            .map(|item| ident(item).to_owned())
            .collect(),
        None => vec![ident(rest).to_owned()],
    }
}

/// The module names `line` of code reaches through `crate::`.
fn paths_in(line: &str) -> Vec<String> {
    line.match_indices("crate::")
        .filter_map(|(at, word)| line.get(at.checked_add(word.len())?..))
        .flat_map(reached)
        .collect()
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
        let text = fs::read_to_string(file)?;
        let code = text.lines().filter(|l| !l.trim().starts_with("//"));
        names.extend(code.flat_map(paths_in));
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
fn a_cycle_is_seen() {
    let edge = |to: &str| BTreeSet::from([to.to_owned()]);
    let graph =
        Graph::from([("a".to_owned(), edge("b")), ("b".to_owned(), edge("a"))]);
    assert!(!acyclic(&graph));
}
