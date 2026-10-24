//! `chert:<ref>`: issued once at insert as the shortest prefix (>= 8
//! chars) of the row's ULID that equals no ref issued in its project,
//! stored, and resolved by exact match only (root I, V15, src/store V8).

#[cfg(test)]
mod tests;

use std::fmt;

use super::Error;

/// The shortest ref (root I, D28).
const MIN: usize = 8;
/// A whole ULID.
const MAX: usize = 26;
/// Crockford base32, as ULIDs are written.
const ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const SCHEME: &str = "chert:";

/// An issued ref, shown as `chert:<ref>`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ref(String);

impl Ref {
    /// `chert:<ref>` as an agent or a skill gives it back: 8 to 26
    /// Crockford base32 characters, uppercase, as issued. Parsing only
    /// checks the form; [`super::Store::get`] matches it exactly.
    ///
    /// # Errors
    /// [`Error::BadRef`] for anything else.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let bad = || Error::BadRef {
            text: text.to_owned(),
        };
        let body = text.strip_prefix(SCHEME).ok_or_else(bad)?;
        let sized = (MIN..=MAX).contains(&body.len());
        if !sized || !body.chars().all(|c| ALPHABET.contains(c)) {
            return Err(bad());
        }
        Ok(Self(body.to_owned()))
    }

    /// The ref as stored, without `chert:`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// A ref read back from the store.
    pub(super) fn stored(text: String) -> Self {
        Self(text)
    }
}

impl fmt::Display for Ref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{SCHEME}{}", self.0)
    }
}

/// The refs `id` may be issued as, shortest first: its prefixes of 8
/// chars up to the whole id.
pub(super) fn candidates(id: &str) -> impl Iterator<Item = &str> {
    (MIN..=id.len()).filter_map(|n| id.get(..n))
}
