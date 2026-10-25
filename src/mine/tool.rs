//! A stored `post-tool-use` body read back: `src/hook` writes the tool
//! name, its input and its response as 1 line each, input and response
//! compact JSON, then compresses the whole at `compression.grammar`.
//! Compression keeps paths and the JSON shape, but may drop words inside
//! a command string; the same command compresses to the same bytes, so
//! repeats still match. A `post-tool-use-failure` body has the error
//! text in place of the response (src/hook R5).

#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

use serde_json::{Map, Value};

/// The `tool_input` keys that name a path, as `src/hook` reads them.
const PATH_KEYS: [&str; 3] = ["file_path", "path", "notebook_path"];

/// The tool whose `command` is a shell command.
const SHELL: &str = "Bash";

/// What mining reads from 1 tool call.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Tool {
    /// The tool name, the 1st line of the body.
    pub(super) name: String,
    /// `tool_input.command` of a `Bash` call.
    pub(super) command: Option<String>,
    /// The paths `tool_input` names, as `src/hook` checks them against
    /// `privacy.exclude`, each once.
    pub(super) paths: Vec<String>,
}

/// `body` as a tool call; `None` when its input line is not a JSON
/// object.
pub(super) fn parse(body: &str) -> Option<Tool> {
    let mut lines = body.split('\n');
    let name = lines.next()?;
    let input: Value = serde_json::from_str(lines.next()?).ok()?;
    let input = input.as_object()?;
    let text = |key| input.get(key).and_then(Value::as_str);
    let command = text("command").filter(|_| name == SHELL);
    Some(Tool {
        name: name.to_owned(),
        command: command.map(str::to_owned),
        paths: paths(input),
    })
}

/// The error of a stored `post-tool-use-failure` body (src/hook R5), as
/// a failure is counted under (src/mine T1): the first non-blank line of
/// the error text, trimmed; empty when it has none. Later lines carry
/// output that changes run to run, the first the stable cause (Claude
/// Code's `Exit code N` for Bash). Numbers stay: exit codes tell
/// failures apart, and a number in a path names another path.
pub(super) fn error(body: &str) -> String {
    let text = body.splitn(3, '\n').nth(2).unwrap_or_default();
    let mut lines = text.lines().map(str::trim);
    lines.find(|l| !l.is_empty()).unwrap_or_default().to_owned()
}

/// Every path `input` names, sorted, each once.
fn paths(input: &Map<String, Value>) -> Vec<String> {
    let found = PATH_KEYS.iter().filter_map(|k| input.get(*k));
    let named = found.filter_map(Value::as_str).map(str::to_owned);
    named.collect::<BTreeSet<_>>().into_iter().collect()
}
