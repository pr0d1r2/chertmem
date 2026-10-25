use super::{Tool, error, parse};
use crate::grammar::{Intensity, compress};

/// A body as `src/hook` builds it, compressed at the default grammar.
fn body(name: &str, input: &str) -> String {
    let raw = format!("{name}\n{input}\n{{\"stdout\":\"the end\"}}");
    compress(&raw, Intensity::Full)
}

#[test]
fn a_bash_call_gives_its_command() {
    let tool = parse(&body("Bash", r#"{"command":"cargo nextest run"}"#));
    let command = tool.and_then(|t| t.command);
    assert_eq!(command.as_deref(), Some("cargo nextest run"));
}

#[test]
fn the_same_command_compresses_to_the_same_text() {
    let input = r#"{"command":"echo the end, really","description":"x"}"#;
    let first = parse(&body("Bash", input));
    assert!(first.as_ref().is_some_and(|t| t.command.is_some()));
    assert_eq!(first, parse(&body("Bash", input)));
}

#[test]
fn a_command_of_another_tool_is_no_command() {
    let tool = parse(&body("Task", r#"{"command":"ls"}"#));
    let name = "Task".to_owned();
    assert_eq!(
        tool,
        Some(Tool {
            name,
            ..Tool::default()
        })
    );
}

#[test]
fn a_call_gives_its_tool_name() {
    let tool = parse(&body("WebSearch", r#"{"query":"rust"}"#));
    assert_eq!(tool.map(|t| t.name).as_deref(), Some("WebSearch"));
}

#[test]
fn every_path_key_is_read_once() {
    let input = r#"{"file_path":"src/lib.rs","path":"src/lib.rs",
        "notebook_path":"nb/b.ipynb"}"#
        .replace('\n', "");
    let tool = parse(&body("Edit", &input));
    let paths = tool.map(|t| t.paths).unwrap_or_default();
    assert_eq!(paths, ["nb/b.ipynb", "src/lib.rs"]);
}

#[test]
fn a_bash_call_names_no_path() {
    let tool = parse(&body("Bash", r#"{"command":"cat src/lib.rs"}"#));
    assert!(tool.is_some_and(|t| t.paths.is_empty()));
}

#[test]
fn a_body_without_a_json_input_is_no_tool() {
    assert_eq!(parse("Bash"), None);
    assert_eq!(parse("Bash\nnot json\n{}"), None);
    assert_eq!(parse("Bash\n[1]\n{}"), None);
}

/// A failure body as `src/hook` builds it (src/hook R5), compressed.
fn failed(error: &str) -> String {
    let raw = format!("Bash\n{{\"command\":\"false\"}}\n{error}");
    compress(&raw, Intensity::Full)
}

#[test]
fn a_failure_is_counted_under_its_errors_first_line() {
    let got = error(&failed("Exit code 101\nerror: test failed, at 12:03"));
    assert_eq!(got, error(&failed("Exit code 101")));
    assert!(got.contains("101"), "{got}");
    assert!(!got.contains('\n'), "{got}");
}

#[test]
fn blank_lines_and_edge_blanks_are_skipped() {
    let got = error(&failed("\n  \n  Exit code 2  \nmore"));
    assert_eq!(got, error(&failed("Exit code 2")));
    assert!(!got.starts_with(' ') && !got.ends_with(' '), "{got:?}");
}

#[test]
fn exit_codes_and_numbers_in_paths_are_kept() {
    let one = error(&failed("Exit code 1"));
    assert_ne!(one, error(&failed("Exit code 2")));
    assert_ne!(one, String::new());
    let (a, b) = ("cannot open tmp/run-1", "cannot open tmp/run-2");
    assert_ne!(error(&failed(a)), error(&failed(b)));
}

#[test]
fn a_body_without_an_error_line_has_an_empty_error() {
    assert_eq!(error("Bash\n{}"), "");
    assert_eq!(error("Bash\n{}\n\n  "), "");
    assert_eq!(error("Bash"), "");
}
