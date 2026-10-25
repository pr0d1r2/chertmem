//! A peek reads the catalog as it stands and changes nothing on disk.

use std::fs;

use super::super::tests::{Scratch, Test, event, stored};
use crate::store::{Filter, Mode, Store};

#[test]
fn no_data_dir_or_no_catalog_is_none_and_makes_nothing() -> Test {
    let s = Scratch::new("peek-none")?;
    let mut cfg = s.config()?;
    assert!(Store::peek(&cfg, &s.project())?.is_none());
    assert!(!s.data().exists());
    cfg.storage.data_dir = None;
    assert!(Store::peek(&cfg, &s.project())?.is_none());
    Ok(())
}

#[test]
fn a_peek_sees_the_catalogs_partitions_and_stats() -> Test {
    let s = Scratch::new("peek-stats")?;
    let mut w = s.open(Mode::Read)?;
    stored(w.append(&event(1_000, None, "one"))?)?;
    stored(w.append(&event(2_000, None, "three"))?)?;
    drop(w);
    let peek = Store::peek(&s.config()?, &s.project())?.ok_or("no catalog")?;
    let filter = Filter {
        project: Some(peek.project().to_owned()),
        ..Filter::default()
    };
    let parts = peek.partitions(&filter)?;
    let rows: u64 = parts.iter().map(|p| p.stats.rows).sum();
    let bytes: u64 = parts.iter().map(|p| p.stats.bytes).sum();
    assert_eq!((rows, bytes > 0), (2, true), "{parts:?}");
    Ok(())
}

#[test]
fn a_peek_leaves_the_spool_and_cannot_write() -> Test {
    let s = Scratch::new("peek-spool")?;
    drop(s.open(Mode::Read)?);
    let held = s.data().join("spool").join("0000-held.jsonl");
    fs::write(&held, "{}\n")?;
    let mut peek = Store::peek(&s.config()?, &s.project())?.ok_or("none")?;
    assert!(held.exists());
    assert!(peek.append(&event(3_000, None, "x")).is_err());
    Ok(())
}
