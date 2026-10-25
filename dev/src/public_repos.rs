//! `dev/public-repos.toml` (dev V14, `paradigm/V37`): the public
//! repositories committed text may name or link, each dated by the
//! `gh repo view` that showed it PUBLIC; `self` is this repository,
//! exempt until it is public. `name-guard` holds every reference to it.

#[cfg(test)]
mod tests;

/// The list, by its committed path.
pub const PATH: &str = "dev/public-repos.toml";

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
        let _ = text;
        Ok(Self::default())
    }

    /// Whether `repo` (`owner/name`) may be named, ignoring case.
    #[must_use]
    pub fn allows(&self, repo: &str) -> bool {
        let _ = (repo, &self.own, &self.repos);
        true
    }
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
    let _ = text;
    Vec::new()
}

/// Every reference in `files` that `allow` does not hold.
#[must_use]
pub fn unlisted(
    allow: &Allowlist,
    files: &[(String, String)],
) -> Vec<Unlisted> {
    let _ = (allow, files);
    Vec::new()
}
