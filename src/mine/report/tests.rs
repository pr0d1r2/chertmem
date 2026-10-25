use serde_json::{Value, json};

use super::{Format, report};
use crate::mine::tests::{Res, Test, reference};
use crate::mine::{Finding, Kind};

fn finding(id: &str, kind: Kind, summary: &str) -> Res<Finding> {
    let refs = vec![reference(1)?, reference(3)?];
    Ok(Finding {
        id: id.to_owned(),
        kind,
        count: refs.len(),
        refs,
        summary: summary.to_owned(),
    })
}

fn two() -> Res<Vec<Finding>> {
    Ok(vec![
        finding("00000000000000aa", Kind::Sequence, "`a | b` then `c`")?,
        finding("00000000000000bb", Kind::HotFile, "hot file `x.rs`")?,
    ])
}

#[test]
fn every_format_round_trips_its_spelling() {
    for f in [Format::Text, Format::Json, Format::Md] {
        assert_eq!(Format::parse(f.as_str()), Some(f));
    }
    assert_eq!(Format::parse("html"), None);
}

#[test]
fn text_is_1_line_per_finding_then_its_refs() -> Test {
    let text = report(&two()?, Format::Text);
    let want = "2x sequence 00000000000000aa `a | b` then `c`\n  \
        chert:00000001 chert:00000003\n\
        2x hot_file 00000000000000bb hot file `x.rs`\n  \
        chert:00000001 chert:00000003\n";
    assert_eq!(text, want);
    Ok(())
}

#[test]
fn json_carries_every_field_of_a_finding() -> Test {
    let text = report(&two()?, Format::Json);
    assert!(text.ends_with('\n'), "{text}");
    let got: Value = serde_json::from_str(&text)?;
    let first = json!({
        "id": "00000000000000aa", "kind": "sequence", "count": 2,
        "refs": ["chert:00000001", "chert:00000003"],
        "summary": "`a | b` then `c`",
    });
    assert_eq!(got.get(0), Some(&first));
    assert_eq!(got.as_array().map(Vec::len), Some(2));
    Ok(())
}

#[test]
fn md_is_a_table_with_pipes_escaped() -> Test {
    let text = report(&two()?, Format::Md);
    let want = "# chert report\n\n\
        | kind | count | finding | refs |\n\
        | --- | --- | --- | --- |\n\
        | sequence | 2 | `a \\| b` then `c` | \
        chert:00000001 chert:00000003 |\n\
        | hot_file | 2 | hot file `x.rs` | \
        chert:00000001 chert:00000003 |\n";
    assert_eq!(text, want);
    Ok(())
}

#[test]
fn no_findings_says_so_in_every_format() {
    assert_eq!(report(&[], Format::Text), "chert: no findings\n");
    assert_eq!(report(&[], Format::Json), "[]\n");
    let md = report(&[], Format::Md);
    assert_eq!(md, "# chert report\n\nNo findings.\n");
}

#[test]
fn the_same_findings_give_the_same_bytes() -> Test {
    for f in [Format::Text, Format::Json, Format::Md] {
        assert_eq!(report(&two()?, f), report(&two()?, f));
    }
    Ok(())
}

#[test]
fn a_failure_is_spelled_failure_in_every_format() -> Test {
    let fail = "failing `Bash` `false`: `Exit code 1`";
    let one = [finding("00000000000000dd", Kind::Failure, fail)?];
    let text = report(&one, Format::Text);
    let head = format!("2x failure 00000000000000dd {fail}\n");
    assert!(text.starts_with(&head), "{text}");
    let got: Value = serde_json::from_str(&report(&one, Format::Json))?;
    let kind = got.get(0).and_then(|f| f.get("kind"));
    assert_eq!(kind, Some(&json!("failure")));
    let md = report(&one, Format::Md);
    assert!(md.contains(&format!("| failure | 2 | {fail} |")), "{md}");
    Ok(())
}

#[test]
fn a_token_sink_is_spelled_token_sink_in_every_format() -> Test {
    let sink = "token sink `Read` `x.rs`: >= 1/10 of tool call tokens";
    let one = [finding("00000000000000cc", Kind::TokenSink, sink)?];
    let text = report(&one, Format::Text);
    assert!(text.starts_with("2x token_sink 00000000000000cc token"));
    let got: Value = serde_json::from_str(&report(&one, Format::Json))?;
    let kind = got.get(0).and_then(|f| f.get("kind"));
    assert_eq!(kind, Some(&json!("token_sink")));
    let md = report(&one, Format::Md);
    assert!(md.contains(&format!("| token_sink | 2 | {sink} |")), "{md}");
    Ok(())
}
