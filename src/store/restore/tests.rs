use std::error::Error as StdError;

use crate::store::tests::{Scratch, Test, WEEK, event};
use crate::store::write::new_id;
use crate::store::{
    Appended, Error, Filter, Lookup, Mode, Observation, Ref, Store,
};

/// An export from another data dir: 3 rows in weeks 0 and 1, 1 of
/// them with an identity, all in 1 ms of week 0 but the last.
fn export(name: &str) -> Result<Vec<Observation>, Box<dyn StdError>> {
    let s = Scratch::new(name)?;
    let mut store = s.open(Mode::Read)?;
    for (ts, identity) in [(5, Some("tool-1")), (5, None), (WEEK + 1, None)] {
        store.append(&event(ts, identity, "body"))?;
    }
    Ok(store.observations(&Filter::default())?)
}

fn restore_all(
    store: &mut Store,
    rows: &[Observation],
) -> Result<Vec<Appended>, Error> {
    rows.iter().map(|o| store.restore(o)).collect()
}

fn refs(rows: &[Observation]) -> Vec<Ref> {
    rows.iter().map(|o| o.reference.clone()).collect()
}

#[test]
fn an_export_restores_with_the_same_rows_and_refs() -> Test {
    let rows = export("restore-same-src")?;
    let s = Scratch::new("restore-same")?;
    let restored = restore_all(&mut s.open(Mode::Write)?, &rows)?;
    let want: Vec<_> = refs(&rows).into_iter().map(Appended::Stored).collect();
    assert_eq!(restored, want);
    let mut store = s.open(Mode::Read)?;
    let found = store.get(&refs(&rows))?;
    let same: Vec<_> = rows.iter().cloned().map(Lookup::Found).collect();
    assert_eq!(found, same);
    assert_eq!(store.observations(&Filter::default())?, rows);
    Ok(())
}

#[test]
fn restoring_twice_adds_nothing() -> Test {
    let rows = export("restore-twice-src")?;
    let s = Scratch::new("restore-twice")?;
    let mut store = s.open(Mode::Read)?;
    let first = restore_all(&mut store, &rows)?;
    assert_eq!(restore_all(&mut store, &rows)?, first);
    assert_eq!(store.observations(&Filter::default())?, rows);
    Ok(())
}

#[test]
fn a_ref_held_by_another_id_is_refused() -> Test {
    let rows = export("restore-taken-src")?;
    let s = Scratch::new("restore-taken")?;
    let mut store = s.open(Mode::Read)?;
    restore_all(&mut store, &rows)?;
    let mut forged = rows.first().ok_or("no row")?.clone();
    forged.id = new_id(forged.ts);
    forged.identity = None;
    let e = store.restore(&forged).err();
    let reference = forged.reference.as_str().to_owned();
    let id = forged.id.clone();
    assert_eq!(e, Some(Error::RefTaken { reference, id }));
    Ok(())
}

#[test]
fn a_sealed_target_refuses_a_restore() -> Test {
    let rows = export("restore-sealed-src")?;
    let s = Scratch::new("restore-sealed")?;
    let mut store = s.open(Mode::Read)?;
    let first = rows.first().ok_or("no row")?;
    store.restore(first)?;
    let path = format!("chunks/{}/19700101-7d.db", first.project);
    store.seal(&path)?;
    let late = rows.get(1).ok_or("no row")?;
    assert_eq!(store.restore(late), Err(Error::ReadOnly { path }));
    Ok(())
}
