//! A stored `post-tool-use` body read back: `src/hook` writes the tool
//! name, its input and its response as 1 line each, input and response
//! compact JSON, then compresses the whole at `compression.grammar`.
//! Compression keeps paths and the JSON shape, but may drop words inside
//! a command string; the same command compresses to the same bytes, so
//! repeats still match.

#[cfg(test)]
mod tests;

/// What mining reads from 1 tool call.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Tool {
    /// `tool_input.command` of a `Bash` call.
    pub(super) command: Option<String>,
    /// The paths `tool_input` names, as `src/hook` checks them against
    /// `privacy.exclude`, each once.
    pub(super) paths: Vec<String>,
}

/// `body` as a tool call; `None` when its input line is not a JSON
/// object.
pub(super) fn parse(body: &str) -> Option<Tool> {
    // RED stub: an empty call for every body.
    (!body.is_empty()).then(|| Tool {
        command: None,
        paths: Vec::new(),
    })
}
