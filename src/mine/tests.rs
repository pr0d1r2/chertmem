//! Mine-wide tests, and the observations the signal tests mine.

use std::error::Error as StdError;

use super::Kind;
use crate::grammar::{Intensity, compress};
use crate::store::{Observation, Ref};

pub(super) type Test = Result<(), Box<dyn StdError>>;
pub(super) type Res<T> = Result<T, Box<dyn StdError>>;

/// Milliseconds of 2026-10-24, the time test events happen around.
pub(super) const NOW: u64 = 1_792_800_000_000;

/// A `post-tool-use` row `n` ms after [`NOW`] in `session`, with the
/// body `src/hook` stores for tool `name` with `input`; its ref is `n`
/// in 8 digits.
pub(super) fn row(
    n: u64,
    session: &str,
    name: &str,
    input: &str,
) -> Res<Observation> {
    let raw = format!("{name}\n{input}\n{{\"stdout\":\"\"}}");
    Ok(Observation {
        reference: reference(n)?,
        id: format!("{n:026}"),
        project: "project".to_owned(),
        ts: NOW.checked_add(n).ok_or("time out of range")?,
        session: session.to_owned(),
        ide: "claude-code".to_owned(),
        kind: "post-tool-use".to_owned(),
        identity: None,
        body: compress(&raw, Intensity::Full),
    })
}

/// The ref of [`row`] `n`.
pub(super) fn reference(n: u64) -> Res<Ref> {
    Ok(Ref::parse(&format!("chert:{n:08}"))?)
}

/// A Bash call of `command` as row `n` in `session`.
pub(super) fn bash(n: u64, session: &str, command: &str) -> Res<Observation> {
    let input = serde_json::json!({ "command": command }).to_string();
    row(n, session, "Bash", &input)
}

#[test]
fn every_kind_round_trips_its_spelling() {
    for kind in [Kind::Sequence, Kind::HotFile] {
        assert_eq!(Kind::parse(kind.as_str()), Some(kind));
    }
    assert_eq!(Kind::parse("error"), None);
}
