//! The clap definition: consistent, every root I verb in it, `--since`
//! read as a span or as ms (T1, V1), and no Unicode-3.0 in the build.

use std::collections::{HashMap, HashSet};

use clap::{ArgMatches, CommandFactory, FromArgMatches, ValueEnum};
use serde_json::Value;

use super::super::tests::{NOW, Res, Test};
use super::{Cli, Command, IdeName, Since, since};
use crate::config::{Span, Unit};
use crate::hook::Ide;

/// Every verb of root I, as `chert <verb>`.
const VERBS: [&str; 21] = [
    "hook",
    "mcp",
    "search",
    "timeline",
    "get",
    "install",
    "uninstall",
    "status",
    "doctor",
    "config",
    "partitions",
    "cache",
    "mine",
    "report",
    "statusline",
    "compress",
    "expand",
    "export",
    "import",
    "completions",
    "man",
];

#[test]
fn the_definition_passes_clap_own_checks() {
    Cli::command().debug_assert();
}

#[test]
fn every_root_verb_is_a_top_level_subcommand() {
    let command = Cli::command();
    let names: Vec<&str> = command
        .get_subcommands()
        .map(clap::Command::get_name)
        .collect();
    assert_eq!(names, VERBS);
}

#[test]
fn since_reads_hours_days_and_months_back_from_now() -> Test {
    // NOW less 2 h, 14 d and 31 d (a month as the cache counts it).
    assert_eq!(since("2h")?.ms(NOW), 1_792_792_800_000);
    assert_eq!(since("14d")?.ms(NOW), 1_791_590_400_000);
    assert_eq!(since("1mo")?.ms(NOW), 1_790_121_600_000);
    let span = Span {
        amount: 3,
        unit: Unit::Days,
    };
    assert_eq!(since("3d")?, Since::Ago(span));
    Ok(())
}

#[test]
fn since_reads_raw_milliseconds_as_a_time() -> Test {
    assert_eq!(since("1792800000000")?, Since::At(NOW));
    assert_eq!(since("5")?.ms(NOW), 5);
    Ok(())
}

#[test]
fn since_refuses_other_forms_naming_them() -> Test {
    let refused = since("yesterday");
    let Err(why) = refused else {
        return Err(format!("accepted: {refused:?}").into());
    };
    assert!(why.contains("<n>mo") && why.contains("yesterday"), "{why}");
    Ok(())
}

#[test]
fn a_span_past_the_epoch_saturates_at_0() -> Test {
    assert_eq!(since("100000mo")?.ms(NOW), 0);
    Ok(())
}

#[test]
fn ide_names_are_the_hook_dialects() {
    let names: Vec<String> = IdeName::value_variants()
        .iter()
        .filter_map(ValueEnum::to_possible_value)
        .map(|v| v.get_name().to_owned())
        .collect();
    assert_eq!(names, [Ide::ClaudeCode.as_str()]);
}

/// Crates whose licence needs Unicode-3.0, which deny.toml allows for
/// these alone, by exception: dev-only, never in the `chert` binary.
const UNICODE_3: [&str; 1] = ["unicode-ident"];

/// The platforms deny.toml's `[graph]` counts.
const TARGETS: [&str; 4] = [
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
];

/// The builder API, not clap derive, keeps the proc-macro tree (and
/// with it Unicode-3.0) out of every build of `chertmem` (src/cli C).
#[test]
fn the_normal_and_build_graph_needs_no_unicode_3_licence() -> Test {
    let meta = metadata()?;
    let names = built(&meta);
    assert!(names.contains("clap"), "no graph: {names:?}");
    let found: Vec<&str> = UNICODE_3
        .into_iter()
        .filter(|c| names.contains(c))
        .collect();
    assert!(found.is_empty(), "chertmem's build graph reaches {found:?}");
    Ok(())
}

/// The name of every package built into `chertmem`.
fn built(meta: &Value) -> HashSet<&str> {
    let root = meta.pointer("/resolve/root").and_then(Value::as_str);
    let ids = reachable(root.into_iter().collect(), &edges(meta));
    let packages = meta.get("packages").and_then(Value::as_array);
    let packages = packages.into_iter().flatten().filter(|p| {
        let id = p.get("id").and_then(Value::as_str);
        id.is_some_and(|i| ids.contains(i))
    });
    packages.filter_map(|p| p.get("name")?.as_str()).collect()
}

/// `cargo metadata` as locked, every feature on, for [`TARGETS`] only:
/// offline, so it reads the lock and the local registry, fast.
fn metadata() -> Res<Value> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = std::process::Command::new(cargo);
    command.current_dir(env!("CARGO_MANIFEST_DIR"));
    command.args(["metadata", "--format-version=1", "--offline"]);
    command.args(["--locked", "--all-features"]);
    for target in TARGETS {
        command.args(["--filter-platform", target]);
    }
    let out = command.output()?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into());
    }
    Ok(serde_json::from_slice(&out.stdout)?)
}

/// Each resolved package's normal and build dependencies, by id.
fn edges(meta: &Value) -> HashMap<&str, Vec<&str>> {
    let nodes = meta.pointer("/resolve/nodes").and_then(Value::as_array);
    let nodes = nodes.into_iter().flatten();
    nodes
        .filter_map(|n| {
            let deps = n.get("deps")?.as_array()?.iter().filter(|d| linked(d));
            let ids = deps.filter_map(|d| d.get("pkg")?.as_str());
            Some((n.get("id")?.as_str()?, ids.collect()))
        })
        .collect()
}

/// A dependency built into the package: any kind but `dev`.
fn linked(dep: &Value) -> bool {
    let kinds = dep.get("dep_kinds").and_then(Value::as_array);
    let mut kinds = kinds.into_iter().flatten();
    kinds.any(|k| k.get("kind").and_then(Value::as_str) != Some("dev"))
}

/// Every package reached from `from` along `edges`.
fn reachable<'a>(
    from: Vec<&'a str>,
    edges: &HashMap<&'a str, Vec<&'a str>>,
) -> HashSet<&'a str> {
    let (mut seen, mut todo) = (HashSet::new(), from);
    while let Some(id) = todo.pop() {
        if seen.insert(id) {
            todo.extend(edges.get(id).into_iter().flatten());
        }
    }
    seen
}

#[test]
fn matches_naming_no_verb_are_refused() {
    let refused = Cli::from_arg_matches(&ArgMatches::default());
    assert!(refused.is_err(), "{refused:?}");
}

#[test]
fn an_update_parses_the_verb_again() -> Test {
    let command = Cli::command_for_update();
    let matches = command.try_get_matches_from(["chert", "man"])?;
    let mut cli = Cli::from_arg_matches(&matches)?;
    let matches = Cli::command().try_get_matches_from(["chert", "mcp"])?;
    cli.update_from_arg_matches(&matches)?;
    assert!(matches!(cli.command, Command::Serve(_)), "{cli:?}");
    Ok(())
}
