use std::error::Error as StdError;

use crate::search::Error;
use crate::search::tests::{BASE, DAY, Test, engine, event, fresh};
use crate::store::{Appended, Filter, Ref, Store};

/// 7 rows of session `s` a day apart, 1 of `t` between them.
fn seven(store: &mut Store) -> Result<Vec<Ref>, Box<dyn StdError>> {
    store.append(&event(BASE + 2 * DAY, "t", "elsewhere"))?;
    let mut refs = Vec::new();
    let mut ts = BASE;
    for _ in 0..7 {
        match store.append(&event(ts, "s", "row"))? {
            Appended::Stored(r) => refs.push(r),
            Appended::Spooled => return Err("spooled".into()),
        }
        ts = ts.saturating_add(DAY);
    }
    Ok(refs)
}

fn refs(entries: &[crate::search::Entry]) -> Vec<Ref> {
    entries.iter().map(|e| e.reference.clone()).collect()
}

fn slice(all: &[Ref], from: usize, to: usize) -> Vec<Ref> {
    all.iter()
        .skip(from)
        .take(to.saturating_sub(from))
        .cloned()
        .collect()
}

#[test]
fn without_around_the_first_rows_of_the_session_in_order() -> Test {
    let (s, cfg, mut store) = fresh("timeline-first")?;
    let all = seven(&mut store)?;
    let mut e = engine(&s, &cfg)?;
    let got = e.timeline("s", None, Some(3))?;
    assert_eq!(refs(&got), slice(&all, 0, 3));
    assert!(got.iter().all(|x| x.kind == "post-tool-use"));
    assert_eq!(got.first().map(|x| x.ts), Some(BASE));
    assert_eq!(refs(&e.timeline("s", None, None)?), all);
    Ok(())
}

#[test]
fn around_centres_on_the_ref_and_stays_inside_the_session() -> Test {
    let (s, cfg, mut store) = fresh("timeline-around")?;
    let all = seven(&mut store)?;
    let mut e = engine(&s, &cfg)?;
    let at = |i: usize| all.get(i).ok_or("ref");
    let mid = e.timeline("s", Some(at(3)?), Some(3))?;
    assert_eq!(refs(&mid), slice(&all, 2, 5));
    let end = e.timeline("s", Some(at(6)?), Some(3))?;
    assert_eq!(refs(&end), slice(&all, 4, 7));
    let start = e.timeline("s", Some(at(0)?), Some(4))?;
    assert_eq!(refs(&start), slice(&all, 0, 4));
    Ok(())
}

#[test]
fn a_dropped_around_is_an_error_naming_it() -> Test {
    let (s, cfg, mut store) = fresh("timeline-dropped")?;
    let all = seven(&mut store)?;
    let first = store.partitions(&Filter::default())?;
    store.drop(&first.first().ok_or("none")?.path)?;
    let gone = all.first().ok_or("ref")?;
    let got = engine(&s, &cfg)?.timeline("s", Some(gone), None);
    let want = Error::Dropped {
        reference: gone.to_string(),
    };
    assert_eq!(got, Err(want));
    Ok(())
}
