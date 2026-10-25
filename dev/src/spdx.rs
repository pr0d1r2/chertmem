//! SPDX license expressions and the license chert takes from each (dev
//! T11, owner 2026-10-25): MIT whenever the expression offers it,
//! otherwise the most permissive option; every `AND` term stays.

#[cfg(test)]
mod tests;

use std::iter::Peekable;
use std::vec::IntoIter;

/// License ids from most to least permissive, MIT first by the owner's
/// rule; an id not listed ranks after all of them.
const RANK: [&str; 18] = [
    "MIT",
    "MIT-0",
    "0BSD",
    "Unlicense",
    "CC0-1.0",
    "BSL-1.0",
    "ISC",
    "BSD-2-Clause",
    "Zlib",
    "BSD-3-Clause",
    "Apache-2.0",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "MPL-2.0",
    "LGPL-2.1-or-later",
    "LGPL-3.0-or-later",
    "GPL-2.0-or-later",
    "GPL-3.0-or-later",
];

type Tokens<'a> = Peekable<IntoIter<&'a str>>;

/// The license taken from `expr`, its ids joined by ` AND `. A `/`
/// reads as `OR`, as in old crate manifests.
///
/// # Errors
/// `expr` is not an SPDX expression.
pub fn choose(expr: &str) -> Result<String, String> {
    let spaced = expr
        .replace('(', " ( ")
        .replace(')', " ) ")
        .replace('/', " OR ");
    let tokens: Vec<&str> = spaced.split_whitespace().collect();
    let mut tokens = tokens.into_iter().peekable();
    let ids = any(&mut tokens).map_err(|e| format!("`{expr}`: {e}"))?;
    match tokens.next() {
        None => Ok(ids.join(" AND ")),
        Some(t) => Err(format!("`{expr}`: unexpected `{t}`")),
    }
}

/// `all (OR all)*`: the best alternative.
fn any(tokens: &mut Tokens<'_>) -> Result<Vec<String>, String> {
    let mut best = all(tokens)?;
    while tokens.next_if_eq(&"OR").is_some() {
        let next = all(tokens)?;
        if key(&next) < key(&best) {
            best = next;
        }
    }
    Ok(best)
}

/// `one (AND one)*`: every term.
fn all(tokens: &mut Tokens<'_>) -> Result<Vec<String>, String> {
    let mut ids = one(tokens)?;
    while tokens.next_if_eq(&"AND").is_some() {
        ids.extend(one(tokens)?);
    }
    Ok(ids)
}

/// `( any )`, or 1 id with an optional `WITH` exception.
fn one(tokens: &mut Tokens<'_>) -> Result<Vec<String>, String> {
    match tokens.next() {
        Some("(") => {
            let ids = any(tokens)?;
            let close = tokens.next_if_eq(&")");
            close.map(|_| ids).ok_or_else(|| "unclosed `(`".to_owned())
        }
        Some(id) if !matches!(id, ")" | "AND" | "OR" | "WITH") => {
            Ok(vec![with(id, tokens)?])
        }
        t => Err(format!("want a license id, found {t:?}")),
    }
}

fn with(id: &str, tokens: &mut Tokens<'_>) -> Result<String, String> {
    if tokens.next_if_eq(&"WITH").is_none() {
        return Ok(id.to_owned());
    }
    let exception = tokens.next().ok_or("no exception after `WITH`")?;
    Ok(format!("{id} WITH {exception}"))
}

/// Lower is better: MIT offered, then the least permissive id's rank,
/// then fewer terms.
fn key(ids: &[String]) -> (bool, usize, usize) {
    let worst = ids.iter().map(|id| rank(id)).max().unwrap_or(0);
    (!ids.iter().any(|id| id == "MIT"), worst, ids.len())
}

fn rank(id: &str) -> usize {
    let base = id.split(' ').next().unwrap_or(id);
    RANK.iter().position(|r| *r == base).unwrap_or(RANK.len())
}
