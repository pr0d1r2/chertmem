use super::{Tool, parse};
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
    assert_eq!(tool, Some(Tool::default()));
}

#[test]
fn every_path_key_is_read_once() {
    let input = r#"{"file_path":"src/lib.rs","path":"src/lib.rs",
        "notebook_path":"a/b.ipynb"}"#
        .replace('\n', "");
    let tool = parse(&body("Edit", &input));
    let paths = tool.map(|t| t.paths).unwrap_or_default();
    assert_eq!(paths, ["a/b.ipynb", "src/lib.rs"]);
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
