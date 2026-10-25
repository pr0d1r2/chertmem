//! The clap definition: consistent, every root I verb in it, `--since`
//! read as a span or as ms (T1, V1).

use clap::{CommandFactory, ValueEnum};

use super::super::tests::{NOW, Test};
use super::{Cli, IdeName, Since, since};
use crate::config::{Span, Unit};
use crate::hook::Ide;

/// Every verb of root I, as `chert <verb>`.
const VERBS: [&str; 20] = [
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
