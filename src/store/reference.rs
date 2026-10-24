//! `chert:<ref>`: issued once at insert from the row's ULID and stored
//! (root I, V15).

#[cfg(test)]
mod tests;

use std::fmt;

/// An issued ref, shown as `chert:<ref>`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ref(String);

impl Ref {
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
        write!(f, "chert:{}", self.0)
    }
}

/// The refs `id` may be issued as, shortest first.
pub(super) fn candidates(id: &str) -> impl Iterator<Item = &str> {
    std::iter::once(id)
}
