//! `commit-msg`: the message git hands the hook names its type, says why,
//! and ends with the paradigm stage as a trailer (dev V11, B4).

#[cfg(test)]
mod tests;

/// git's scissors line: `git commit -v` appends the diff below it, and
/// cleanup removes everything from here on.
const SCISSORS: &str = "# ------------------------ >8 ------------------------";

const TYPES: [&str; 9] = [
    "feat", "fix", "docs", "test", "chore", "refactor", "perf", "build", "ci",
];

/// git's own subjects, which pass untouched.
const GIT: [&str; 4] = ["Merge ", "Revert ", "fixup! ", "squash! "];

/// The longest subject, in characters.
const LIMIT: usize = 72;

/// The stage trailer's key (`paradigm/V36`).
const KEY: &str = "Paradigm-Stage:";

/// The paradigm stages without a round number (`paradigm` section I).
const STAGES: [&str; 8] = [
    "intent",
    "spec-run",
    "dev-shell",
    "guardrails",
    "gate",
    "release",
    "backprop",
    "propagation",
];

/// The stages that carry a positive round number after the prefix.
const NUMBERED: [&str; 4] =
    ["gap-round-", "ck-check-round-", "wave-", "reconcile-"];

/// Every rule `text` breaks, each with its fix; empty when it holds.
/// Merge, revert, fixup and squash subjects are git's and pass.
#[must_use]
pub fn findings(text: &str) -> Vec<String> {
    let lines = kept(text);
    let subject = lines.first().copied().unwrap_or_default();
    if GIT.iter().any(|p| subject.starts_with(p)) {
        return Vec::new();
    }
    let (body, trailers) = split(&lines);
    [style(subject), length(subject), said(body), stage(trailers)]
        .into_iter()
        .flatten()
        .collect()
}

/// The lines git keeps: up to the scissors line, `#` comment lines
/// dropped, trailing blank lines trimmed.
fn kept(text: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = text
        .lines()
        .take_while(|l| *l != SCISSORS)
        .filter(|l| !l.starts_with('#'))
        .collect();
    let end = lines
        .iter()
        .rposition(|l| !l.trim().is_empty())
        .map_or(0, |i| i.saturating_add(1));
    lines.truncate(end);
    lines
}

/// The lines after the subject, as the body and the final paragraph's
/// trailers; no trailers when git would not parse that paragraph as them.
fn split<'a>(lines: &'a [&'a str]) -> (&'a [&'a str], &'a [&'a str]) {
    let rest = lines.get(1..).unwrap_or_default();
    let Some(blank) = rest.iter().rposition(|l| l.trim().is_empty()) else {
        return (rest, &[]);
    };
    let body = rest.get(..blank).unwrap_or_default();
    let last = rest.get(blank.saturating_add(1)..).unwrap_or_default();
    if trailers(last) {
        (body, last)
    } else {
        (rest, &[])
    }
}

/// Whether git parses `paragraph` as trailers: `Key: value` lines, with
/// indented continuation lines after the first.
fn trailers(paragraph: &[&str]) -> bool {
    paragraph.first().is_some_and(|l| trailer(l))
        && paragraph
            .iter()
            .all(|l| trailer(l) || l.starts_with([' ', '\t']))
}

fn trailer(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, _)| {
        !key.is_empty()
            && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    })
}

fn style(subject: &str) -> Option<String> {
    let types = TYPES.join(" ");
    (!styled(subject)).then(|| {
        format!(
            "subject must be `type(scope): claim`, type one of {types}; \
             got `{subject}`"
        )
    })
}

/// `type(scope)!: claim`: a known type, an optional lower-case scope, an
/// optional `!`, then `: ` and a claim that does not start with a space.
fn styled(subject: &str) -> bool {
    let Some(rest) = TYPES.iter().find_map(|t| subject.strip_prefix(t)) else {
        return false;
    };
    let Some(rest) = scoped(rest) else {
        return false;
    };
    let rest = rest.strip_prefix('!').unwrap_or(rest);
    rest.strip_prefix(": ")
        .is_some_and(|claim| !claim.is_empty() && !claim.starts_with(' '))
}

/// The text after an optional `(scope)`; `None` when it is malformed.
fn scoped(rest: &str) -> Option<&str> {
    let Some(inner) = rest.strip_prefix('(') else {
        return Some(rest);
    };
    let (scope, after) = inner.split_once(')')?;
    (!scope.is_empty() && scope.chars().all(scope_char)).then_some(after)
}

fn scope_char(c: char) -> bool {
    c.is_ascii_lowercase()
        || c.is_ascii_digit()
        || matches!(c, '/' | '.' | '_' | '-')
}

fn length(subject: &str) -> Option<String> {
    let n = subject.chars().count();
    (n > LIMIT).then(|| {
        format!(
            "subject is {n} characters, the limit is {LIMIT}; say less, or \
             move the detail to the body"
        )
    })
}

fn said(body: &[&str]) -> Option<String> {
    (!body.iter().any(|l| !l.trim().is_empty())).then(|| {
        "the body is empty; between the subject and the trailers, say what \
         changed and why"
            .to_owned()
    })
}

/// The finding when no trailer git would parse names the stage.
const NO_STAGE: &str = "the final paragraph holds no `Paradigm-Stage: \
                        <stage>` trailer git would parse; end the message \
                        with a blank line, then the trailer beside the \
                        other `Key: value` trailers";

fn stage(trailers: &[&str]) -> Option<String> {
    let Some(value) = trailers.iter().find_map(|l| l.strip_prefix(KEY)) else {
        return Some(NO_STAGE.to_owned());
    };
    let value = value.trim();
    (!is_stage(value)).then(|| {
        format!(
            "{KEY} {value} is not a paradigm stage; use one of {}, or \
             {}<n> with n a positive integer",
            STAGES.join(" "),
            NUMBERED.join("<n> ")
        )
    })
}

fn is_stage(value: &str) -> bool {
    STAGES.contains(&value)
        || NUMBERED
            .iter()
            .any(|p| value.strip_prefix(p).is_some_and(positive))
}

/// A positive integer written without a leading zero.
fn positive(n: &str) -> bool {
    !n.is_empty()
        && !n.starts_with('0')
        && n.bytes().all(|b| b.is_ascii_digit())
}
