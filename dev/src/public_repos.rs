//! `dev/public-repos.toml` (dev V14, `paradigm/V37`): the public
//! repositories committed text may name or link, each dated by the
//! `gh repo view` that showed it PUBLIC; `self` is this repository,
//! exempt until it is public. `name-guard` holds every reference to it.

#[cfg(test)]
mod tests;

use toml::{Table, Value};

/// The list, by its committed path.
pub const PATH: &str = "dev/public-repos.toml";

/// Prefixes after which an `owner/name` follows.
const PREFIXES: [&str; 2] = ["github.com/", "github:"];

/// The fleet's owner, named in paths with no host in front.
const OWNER: &str = "pr0d1r2/";

/// Repositories committed text may name, lowercased.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Allowlist {
    own: Option<String>,
    repos: Vec<String>,
}

impl Allowlist {
    /// The list in `text`.
    ///
    /// # Errors
    /// Text that is not TOML, an unknown key, or a row without a `name`
    /// or a `YYYY-MM-DD` `public` date, named by its place.
    pub fn parse(text: &str) -> Result<Self, String> {
        let table: Table =
            text.parse().map_err(|e| format!("not TOML: {e}"))?;
        only(&table, &["self", "repo"])?;
        let own = match table.get("self") {
            Some(v) => Some(v.as_str().ok_or("`self` is not a string")?),
            None => None,
        };
        let repos = rows(table.get("repo"))?
            .iter()
            .zip(1_usize..)
            .map(|(v, n)| repo(v).map_err(|e| format!("repo {n}: {e}")))
            .collect::<Result<_, _>>()?;
        let own = own.map(str::to_lowercase);
        Ok(Self { own, repos })
    }

    /// Whether `repo` (`owner/name`) may be named, ignoring case.
    #[must_use]
    pub fn allows(&self, repo: &str) -> bool {
        let repo = repo.to_lowercase();
        self.own.as_deref() == Some(repo.as_str()) || self.repos.contains(&repo)
    }
}

/// An error naming the first key of `table` not in `keys`.
fn only(table: &Table, keys: &[&str]) -> Result<(), String> {
    match table.keys().find(|k| !keys.contains(&k.as_str())) {
        Some(key) => Err(format!("unknown key `{key}`")),
        None => Ok(()),
    }
}

/// The `[[repo]]` tables; none when there are none.
fn rows(value: Option<&Value>) -> Result<&[Value], String> {
    let Some(value) = value else {
        return Ok(&[]);
    };
    let rows = value.as_array().ok_or("`repo` is not an array of tables")?;
    Ok(rows.as_slice())
}

/// One `[[repo]]` row's lowercased name, once its date is checked.
fn repo(value: &Value) -> Result<String, String> {
    let row = value.as_table().ok_or("not a table")?;
    only(row, &["name", "public"])?;
    let text = |key: &str| row.get(key).and_then(Value::as_str);
    let name = text("name").ok_or("no `name` string")?;
    let public = text("public")
        .ok_or_else(|| format!("`{name}` has no `public` date"))?;
    if !is_date(public) {
        return Err(format!(
            "`{name}`: `public` must be YYYY-MM-DD, not `{public}`"
        ));
    }
    Ok(name.to_lowercase())
}

fn is_date(text: &str) -> bool {
    let shape = |(at, c): (usize, char)| match at {
        4 | 7 => c == '-',
        _ => c.is_ascii_digit(),
    };
    text.len() == 10 && text.char_indices().all(shape)
}

/// One reference to a repository the list does not hold.
#[derive(Debug, PartialEq, Eq)]
pub struct Unlisted {
    pub path: String,
    pub line: usize,
    pub repo: String,
}

/// Every `owner/name` in `text`, with its 1-based line: after
/// `github.com/` or `github:`, and every `pr0d1r2/` path.
#[must_use]
pub fn refs(text: &str) -> Vec<(usize, String)> {
    text.lines()
        .zip(1..)
        .flat_map(|(line, n)| in_line(line).into_iter().map(move |r| (n, r)))
        .collect()
}

/// The references in 1 line, each once even when an owner path sits
/// inside a link.
fn in_line(line: &str) -> Vec<String> {
    let after = PREFIXES.iter().flat_map(|p| {
        line.match_indices(p)
            .map(move |(at, _)| at.saturating_add(p.len()))
    });
    let owned = line.match_indices(OWNER).map(|(at, _)| at);
    let mut starts: Vec<usize> = after.chain(owned).collect();
    starts.sort_unstable();
    starts.dedup();
    let mut found: Vec<String> = starts
        .into_iter()
        .filter_map(|at| line.get(at..).and_then(owner_name))
        .collect();
    found.dedup();
    found
}

/// The `owner/name` `rest` starts with.
fn owner_name(rest: &str) -> Option<String> {
    let mut parts = rest.splitn(3, '/');
    let owner = word(parts.next()?)?;
    let name = word(parts.next()?)?;
    Some(format!("{owner}/{name}"))
}

/// The leading run of name characters, without a trailing `.` or `.git`.
fn word(text: &str) -> Option<&str> {
    let allowed =
        |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-');
    let end = text.find(|c: char| !allowed(c)).unwrap_or(text.len());
    let run = text.get(..end)?.trim_end_matches('.');
    let run = run.strip_suffix(".git").unwrap_or(run);
    (!run.is_empty()).then_some(run)
}

/// Every reference in `files` that `allow` does not hold.
#[must_use]
pub fn unlisted(
    allow: &Allowlist,
    files: &[(String, String)],
) -> Vec<Unlisted> {
    files
        .iter()
        .flat_map(|(path, text)| {
            refs(text).into_iter().map(move |(line, repo)| Unlisted {
                path: path.clone(),
                line,
                repo,
            })
        })
        .filter(|u| !allow.allows(&u.repo))
        .collect()
}
