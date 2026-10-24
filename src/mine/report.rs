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
    match format {
        Format::Text => text(findings),
        Format::Json => json(findings),
        Format::Md => md(findings),
    }
}

/// `<count>x <kind> <id> <summary>`, then the refs indented.
fn text(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return "chert: no findings\n".to_owned();
    }
    let block = |f: &Finding| {
        let (kind, refs) = (f.kind.as_str(), refs(f));
        format!("{}x {kind} {} {}\n  {refs}\n", f.count, f.id, f.summary)
    };
    findings.iter().map(block).collect()
}

/// 1 array, keys sorted (`serde_json`'s map is a `BTreeMap`).
fn json(findings: &[Finding]) -> String {
    let one = |f: &Finding| {
        let refs: Vec<String> =
            f.refs.iter().map(ToString::to_string).collect();
        serde_json::json!({
            "id": f.id, "kind": f.kind.as_str(), "count": f.count,
            "refs": refs, "summary": f.summary,
        })
    };
    let all: Vec<serde_json::Value> = findings.iter().map(one).collect();
    format!("{}\n", serde_json::Value::Array(all))
}

/// A heading, then 1 table row per finding.
fn md(findings: &[Finding]) -> String {
    let head = "# chert report\n\n";
    if findings.is_empty() {
        return format!("{head}No findings.\n");
    }
    let row = |f: &Finding| {
        let summary = f.summary.replace('|', "\\|");
        let (kind, refs) = (f.kind.as_str(), refs(f));
        format!("| {kind} | {} | {summary} | {refs} |\n", f.count)
    };
    let rows: String = findings.iter().map(row).collect();
    let table =
        "| kind | count | finding | refs |\n| --- | --- | --- | --- |\n";
    format!("{head}{table}{rows}")
}

/// The refs of `finding`, `chert:<ref>` each, blank-separated.
fn refs(finding: &Finding) -> String {
    let all: Vec<String> =
        finding.refs.iter().map(ToString::to_string).collect();
    all.join(" ")
}
