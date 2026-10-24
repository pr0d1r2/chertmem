use crate::store::tests::{Scratch, Test, WEEK, chunk, event, stored};
use crate::store::{Error, Filter, Mode, Ref, Tailed};

fn refs(rows: &[Tailed]) -> Vec<Ref> {
    rows.iter()
        .map(|t| t.observation.reference.clone())
        .collect()
}

#[test]
fn catalog_version_moves_on_another_connections_commit() -> Test {
    let s = Scratch::new("tail-catalog")?;
    let cache = s.open(Mode::Read)?;
    let before = cache.catalog_version()?;
    s.open(Mode::Write)?.append(&event(5, None, "elsewhere"))?;
    assert_ne!(cache.catalog_version()?, before);
    Ok(())
}

#[test]
fn data_version_moves_on_another_connections_commit() -> Test {
    let s = Scratch::new("tail-data")?;
    let mut cache = s.open(Mode::Read)?;
    cache.append(&event(5, None, "first"))?;
    let path = chunk(&cache, "19700101-7d");
    let before = cache.data_version(&path)?.ok_or("no file")?;
    s.open(Mode::Write)?.append(&event(6, None, "second"))?;
    assert_ne!(cache.data_version(&path)?, Some(before));
    assert_eq!(cache.data_version(&chunk(&cache, "nowhere"))?, None);
    Ok(())
}

#[test]
fn tail_reads_by_seq_past_after() -> Test {
    let s = Scratch::new("tail-seq")?;
    let mut store = s.open(Mode::Read)?;
    // A later ts appended first: seq order, not ULID order.
    let a = stored(store.append(&event(9, None, "late ts"))?)?;
    let b = stored(store.append(&event(2, None, "early ts"))?)?;
    let c = stored(store.append(&event(4, None, "middle"))?)?;
    let path = chunk(&store, "19700101-7d");
    let all = store.tail(&path, 0, &Filter::default())?;
    assert_eq!(refs(&all), [a, b.clone(), c.clone()]);
    let first = all.first().ok_or("empty")?.seq;
    let rest = store.tail(&path, first, &Filter::default())?;
    assert_eq!(refs(&rest), [b, c]);
    Ok(())
}

#[test]
fn tail_applies_the_filter_and_a_gone_file_is_empty() -> Test {
    let s = Scratch::new("tail-filter")?;
    let mut store = s.open(Mode::Read)?;
    store.append(&event(2, None, "early"))?;
    let late = stored(store.append(&event(WEEK - 1, None, "late"))?)?;
    let path = chunk(&store, "19700101-7d");
    let since = Filter {
        since: Some(3),
        ..Filter::default()
    };
    assert_eq!(refs(&store.tail(&path, 0, &since)?), [late]);
    let gone = store.tail(&chunk(&store, "nowhere"), 0, &since)?;
    assert!(gone.is_empty());
    Ok(())
}

#[test]
fn a_write_only_store_refuses_every_tail_read() -> Test {
    let s = Scratch::new("tail-write")?;
    let mut hook = s.open(Mode::Write)?;
    let path = "chunks/x/19700101-7d.db";
    assert_eq!(hook.catalog_version(), Err(Error::WriteOnly));
    assert_eq!(hook.data_version(path), Err(Error::WriteOnly));
    let got = hook.tail(path, 0, &Filter::default());
    assert_eq!(got, Err(Error::WriteOnly));
    Ok(())
}
