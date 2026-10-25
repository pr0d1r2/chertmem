//! Compress & expand over every event body a fixture generates, as the
//! hooks store them at the default `compression.grammar = "full"`.

#[cfg(test)]
mod tests;

use std::hint::black_box;

use chertmem::grammar::{Intensity, compress as squeeze, expand as grow};

use crate::fixture::{Fixture, Res, Row, Rows};
use crate::suites::Op;

/// The plain body of every event of `fx`'s spec; none is an error.
pub(crate) fn bodies(fx: &Fixture) -> Res<Vec<String>> {
    let events = Rows::new(fx.spec).filter_map(|row| match row {
        Row::Event(e) => Some(e.body),
        Row::Begin(_) | Row::Summary(_) => None,
    });
    let all: Vec<String> = events.collect();
    if all.is_empty() {
        return Err("a text suite needs at least 1 event".into());
    }
    Ok(all)
}

/// Compress every body.
pub(crate) fn compress(fx: &Fixture) -> Res<Op> {
    let plain = bodies(fx)?;
    Ok(Box::new(move || {
        for body in &plain {
            black_box(squeeze(body, Intensity::Full));
        }
        Ok(())
    }))
}

/// Expand every body, compressed first.
pub(crate) fn expand(fx: &Fixture) -> Res<Op> {
    let plain = bodies(fx)?;
    let short: Vec<String> =
        plain.iter().map(|b| squeeze(b, Intensity::Full)).collect();
    Ok(Box::new(move || {
        for body in &short {
            black_box(grow(body));
        }
        Ok(())
    }))
}
