//! The Claude Code dialect: its hook input JSON as a [`Payload`]
//! (src/hook C, D24). Unknown fields are ignored.

#[cfg(test)]
mod tests;

use super::{Kind, Payload};

/// `raw` as a payload of `kind`; `None` when it is not a JSON object
/// with a non-empty `session_id` string.
pub(super) fn parse(_kind: Kind, _raw: &[u8]) -> Option<Payload> {
    None
}
