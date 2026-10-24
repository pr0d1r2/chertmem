//! cavemem's `lexicon.json`, every list in its original order, and the
//! alternations built from it once (src/grammar C: compile once).

#[cfg(test)]
mod tests;

use std::cmp::Reverse;
use std::sync::LazyLock;

use super::Intensity;

type Pairs = &'static [(&'static str, &'static str)];

const FILLERS_LITE: &[&str] = &[
    "basically",
    "really",
    "just",
    "actually",
    "simply",
    "literally",
    "quite",
    "rather",
    "very",
    "kind of",
    "sort of",
    "pretty much",
];
const FILLERS_FULL: &[&str] = &[
    "basically",
    "really",
    "just",
    "actually",
    "simply",
    "literally",
    "quite",
    "rather",
    "very",
    "kind of",
    "sort of",
    "pretty much",
    "in order to",
    "it should be noted that",
    "as a matter of fact",
    "please note that",
    "i think that",
    "i believe that",
    "it seems that",
    "it appears that",
];
const FILLERS_ULTRA: &[&str] = &[
    "basically",
    "really",
    "just",
    "actually",
    "simply",
    "literally",
    "quite",
    "rather",
    "very",
    "kind of",
    "sort of",
    "pretty much",
    "in order to",
    "it should be noted that",
    "as a matter of fact",
    "please note that",
    "i think that",
    "i believe that",
    "it seems that",
    "it appears that",
    "perhaps",
    "possibly",
    "probably",
    "generally",
    "typically",
    "usually",
];
const ARTICLES: &[&str] = &["the", "a", "an"];
const HEDGES_FULL: &[&str] = &[
    "i would recommend",
    "we should probably",
    "it might be",
    "might want to",
    "could potentially",
    "may possibly",
];
const HEDGES_ULTRA: &[&str] = &[
    "i would recommend",
    "we should probably",
    "it might be",
    "might want to",
    "could potentially",
    "may possibly",
    "i think",
    "i believe",
    "probably",
    "maybe",
    "perhaps",
];
const PLEASANTRIES_LITE: &[&str] = &[
    "please",
    "thanks",
    "thank you",
    "sorry",
    "sure",
    "of course",
    "no problem",
];
const PLEASANTRIES_FULL: &[&str] = &[
    "please",
    "thanks",
    "thank you",
    "sorry",
    "sure",
    "of course",
    "no problem",
    "happy to help",
];
const ABBREVIATIONS_LITE: Pairs = &[
    ("configuration", "config"),
    ("implementation", "impl"),
    ("documentation", "docs"),
    ("repository", "repo"),
    ("repositories", "repos"),
];
/// The abbreviations `full` and `ultra` share after their first entry.
const SHARED: Pairs = &[
    ("implementation", "impl"),
    ("documentation", "docs"),
    ("repository", "repo"),
    ("repositories", "repos"),
    ("database", "db"),
    ("databases", "dbs"),
    ("application", "app"),
    ("applications", "apps"),
    ("environment", "env"),
    ("environments", "envs"),
    ("dependency", "dep"),
    ("dependencies", "deps"),
    ("directory", "dir"),
    ("directories", "dirs"),
    ("parameter", "param"),
    ("parameters", "params"),
    ("argument", "arg"),
    ("arguments", "args"),
    ("function", "fn"),
    ("functions", "fns"),
    ("variable", "var"),
    ("variables", "vars"),
    ("reference", "ref"),
    ("references", "refs"),
    ("authentication", "auth"),
    ("authorization", "authz"),
    ("middleware", "mw"),
    ("request", "req"),
    ("response", "resp"),
    ("message", "msg"),
    ("messages", "msgs"),
    ("transaction", "tx"),
    ("transactions", "txs"),
    ("asynchronous", "async"),
    ("synchronous", "sync"),
];
/// The abbreviations only `ultra` makes, after `SHARED`.
const ULTRA_ONLY: Pairs = &[
    ("because", "b/c"),
    ("with", "w/"),
    ("without", "w/o"),
    ("should", "shld"),
    ("would", "wld"),
    ("could", "cld"),
    ("number", "num"),
    ("between", "btwn"),
    ("through", "thru"),
    ("and", "&"),
    ("before", "b4"),
    ("maximum", "max"),
    ("minimum", "min"),
];

/// The grammar of one intensity, ready to apply.
#[derive(Debug)]
pub(super) struct Grammar {
    /// Phrase alternations removed in turn: pleasantries, hedges,
    /// fillers, articles; each sorted longest first, as `removePhrases`.
    pub(super) removals: [Vec<&'static str>; 4],
    /// Word abbreviations applied one by one, in `lexicon.json` order:
    /// the pairs of each slice, the slices in turn.
    pub(super) abbreviations: [Pairs; 3],
}

/// `[...phrases].sort((a, b) => b.length - a.length)`: stable.
fn longest_first(phrases: &[&'static str]) -> Vec<&'static str> {
    let mut sorted = phrases.to_vec();
    sorted.sort_by_key(|p| Reverse(p.len()));
    sorted
}

fn grammar(lists: [&[&'static str]; 4], abbreviations: [Pairs; 3]) -> Grammar {
    Grammar {
        removals: lists.map(longest_first),
        abbreviations,
    }
}

static LITE: LazyLock<Grammar> = LazyLock::new(|| {
    let lists = [PLEASANTRIES_LITE, &[], FILLERS_LITE, &[]];
    grammar(lists, [ABBREVIATIONS_LITE, &[], &[]])
});
static FULL: LazyLock<Grammar> = LazyLock::new(|| {
    let lists = [PLEASANTRIES_FULL, HEDGES_FULL, FILLERS_FULL, ARTICLES];
    grammar(lists, [&[("configuration", "config")], SHARED, &[]])
});
static ULTRA: LazyLock<Grammar> = LazyLock::new(|| {
    let lists = [PLEASANTRIES_FULL, HEDGES_ULTRA, FILLERS_ULTRA, ARTICLES];
    grammar(lists, [&[("configuration", "cfg")], SHARED, ULTRA_ONLY])
});

/// The grammar of `intensity`; none for `Off`.
pub(super) fn grammar_for(intensity: Intensity) -> Option<&'static Grammar> {
    match intensity {
        Intensity::Off => None,
        Intensity::Lite => Some(&LITE),
        Intensity::Full => Some(&FULL),
        Intensity::Ultra => Some(&ULTRA),
    }
}

const EXPANSIONS: Pairs = &[
    ("impl", "implementation"),
    ("config", "configuration"),
    ("cfg", "configuration"),
    ("docs", "documentation"),
    ("repo", "repository"),
    ("repos", "repositories"),
    ("db", "database"),
    ("dbs", "databases"),
    ("app", "application"),
    ("apps", "applications"),
    ("env", "environment"),
    ("envs", "environments"),
    ("dep", "dependency"),
    ("deps", "dependencies"),
    ("dir", "directory"),
    ("dirs", "directories"),
    ("param", "parameter"),
    ("params", "parameters"),
    ("arg", "argument"),
    ("args", "arguments"),
    ("fn", "function"),
    ("fns", "functions"),
    ("var", "variable"),
    ("vars", "variables"),
    ("ref", "reference"),
    ("refs", "references"),
    ("auth", "authentication"),
    ("authz", "authorization"),
    ("mw", "middleware"),
    ("req", "request"),
    ("resp", "response"),
    ("msg", "message"),
    ("msgs", "messages"),
    ("tx", "transaction"),
    ("txs", "transactions"),
    ("b/c", "because"),
    ("w/", "with"),
    ("w/o", "without"),
    ("shld", "should"),
    ("wld", "would"),
    ("cld", "could"),
    ("num", "number"),
    ("btwn", "between"),
    ("thru", "through"),
    ("b4", "before"),
    ("max", "maximum"),
    ("min", "minimum"),
];

/// `expand`'s keys sorted longest first, each with its expansion.
static EXPAND: LazyLock<(Vec<&'static str>, Vec<&'static str>)> =
    LazyLock::new(|| {
        let mut pairs = EXPANSIONS.to_vec();
        pairs.sort_by_key(|(key, _)| Reverse(key.len()));
        pairs.into_iter().unzip()
    });

/// The expansion keys, longest first, and their targets by index.
pub(super) fn expansions() -> (&'static [&'static str], &'static [&'static str])
{
    (&EXPAND.0, &EXPAND.1)
}
