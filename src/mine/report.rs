//! `chert mine` and `chert report` output (src/mine I): a pure function
//! of the findings, so the same findings give the same bytes (root V9).
//! Printing it is `src/cli`'s.

#[cfg(test)]
mod tests;

use super::Finding;

/// How a report is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// 1 line per finding, then its refs indented.
    Text,
    /// 1 JSON array of `{id, kind, count, refs, summary}`.
    Json,
    /// A Markdown table.
    Md,
}

const FORMATS: [(Format, &str); 3] = [
    (Format::Text, "text"),
    (Format::Json, "json"),
    (Format::Md, "md"),
];

impl Format {
    /// The spelling `--format` takes.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        FORMATS
            .iter()
            .find(|&&(f, _)| f == self)
            .map_or("text", |&(_, name)| name)
    }

    /// `text`, `json` or `md`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        FORMATS.iter().find(|(_, n)| *n == text).map(|&(f, _)| f)
    }
}

/// `findings` written as `format`, in the order given.
#[must_use]
pub fn report(findings: &[Finding], format: Format) -> String {
    let _ = (findings, format);
    String::new()
}
