use std::error::Error as StdError;

use super::new_id;
use crate::store::tests::{Scratch, Test, WEEK, chunk, event, stored};
use crate::store::{Error, Mode};

/// Every (ts, ref) row of the partition file at `path`.
fn rows(
    s: &Scratch,
    path: &str,
) -> Result<Vec<(u64, String)>, Box<dyn StdError>> {
    let conn = s.sql(path)?;
    let mut stmt =
        conn.prepare("SELECT ts, ref FROM observations ORDER BY seq")?;
    let found = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(found.collect::<Result<_, _>>()?)
}

#[test]
fn ids_are_ulids_at_the_event_time() -> Test {
    let ts = 1_709_211_600_000;
    let (a, b) = (new_id(ts), new_id(ts));
    assert_ne!(a, b);
    assert_eq!(ulid::Ulid::from_string(&a)?.timestamp_ms(), ts);
    // 10 chars of time: same millisecond, same first 10 chars.
    assert_eq!((a.len(), a.get(..10)), (26, b.get(..10)));
    Ok(())
}

#[test]
fn a_row_lands_in_the_bucket_of_its_own_time() -> Test {
    let s = Scratch::new("own-ts")?;
    let mut store = s.open(Mode::Write)?;
    let first = stored(store.append(&event(WEEK - 1, None, "a"))?)?;
    let second = stored(store.append(&event(WEEK, None, "b"))?)?;
    let old = rows(&s, &chunk(&store, "19700101-7d"))?;
    let new = rows(&s, &chunk(&store, "19700108-7d"))?;
    assert_eq!(old, [(WEEK - 1, first.as_str().to_owned())]);
    assert_eq!(new, [(WEEK, second.as_str().to_owned())]);
    Ok(())
}

#[test]
fn the_same_identity_twice_is_one_row_and_one_ref() -> Test {
    let s = Scratch::new("identity")?;
    let mut store = s.open(Mode::Write)?;
    let first = stored(store.append(&event(5, Some("toolu_1"), "a"))?)?;
    // A resend may carry another time: the identity still wins.
    let resend = event(WEEK + 5, Some("toolu_1"), "a");
    assert_eq!(stored(store.append(&resend)?)?, first);
    assert_eq!(rows(&s, &chunk(&store, "19700101-7d"))?.len(), 1);
    assert!(!s.data().join(chunk(&store, "19700108-7d")).exists());
    Ok(())
}

#[test]
fn an_event_without_identity_is_a_new_row_every_time() -> Test {
    let s = Scratch::new("no-identity")?;
    let mut store = s.open(Mode::Write)?;
    let a = stored(store.append(&event(5, None, "cargo test"))?)?;
    let b = stored(store.append(&event(5, None, "cargo test"))?)?;
    assert_ne!(a, b);
    assert_eq!(rows(&s, &chunk(&store, "19700101-7d"))?.len(), 2);
    Ok(())
}

type CatalogRow = (String, u64, u64, u64, u64, String, String);

#[test]
fn the_catalog_counts_rows_and_bytes_under_the_spec() -> Test {
    let s = Scratch::new("catalog-row")?;
    let mut store = s.open(Mode::Write)?;
    store.append(&event(5, None, "abc"))?;
    store.append(&event(6, None, "de"))?;
    let sql = "SELECT dims, t0, t1, rows, bytes, state, spec FROM partitions";
    let got: CatalogRow = s.sql("catalog.db")?.query_row(sql, [], |r| {
        let text = |i| r.get::<_, String>(i);
        let n = |i| r.get::<_, u64>(i);
        Ok((text(0)?, n(1)?, n(2)?, n(3)?, n(4)?, text(5)?, text(6)?))
    })?;
    let spec = "by=project,time;time_bucket=7d;shards=1".to_owned();
    let dims = "project,time".to_owned();
    assert_eq!(got, (dims, 0, WEEK, 2, 5, "hot".to_owned(), spec));
    Ok(())
}

#[test]
fn a_sealed_partition_takes_no_write() -> Test {
    let s = Scratch::new("sealed")?;
    let mut store = s.open(Mode::Write)?;
    store.append(&event(5, None, "a"))?;
    let sql = "UPDATE partitions SET state = 'cold'";
    s.sql("catalog.db")?.execute(sql, [])?;
    let e = store.append(&event(6, None, "b")).err();
    assert!(matches!(e, Some(Error::ReadOnly { .. })), "{e:?}");
    Ok(())
}

#[test]
fn a_timestamp_past_48_bits_is_refused() -> Test {
    let s = Scratch::new("bad-ts")?;
    let mut store = s.open(Mode::Write)?;
    let e = store.append(&event(1 << 48, None, "a")).err();
    assert_eq!(e, Some(Error::BadTime { ts: 1 << 48 }));
    Ok(())
}

#[test]
fn an_id_with_every_candidate_issued_is_refused() -> Test {
    let s = Scratch::new("issue-exhausted")?;
    drop(s.open(Mode::Read)?);
    let conn = s.sql("catalog.db")?;
    conn.execute_batch(
        "INSERT INTO partitions (path, dims, project, ide, t0, t1, checksum,
           spec) VALUES ('a', 'd', 'p', '', 0, 1, 'c', 's');
         INSERT INTO refs (project, ref, id, partition)
           VALUES ('p', 'ABCDEFGH', 'X', 1);",
    )?;
    // An 8-char id has 1 candidate: itself.
    assert_eq!(super::issue(&conn, "q", "ABCDEFGH")?, "ABCDEFGH");
    let e = super::issue(&conn, "p", "ABCDEFGH");
    assert!(matches!(e, Err(Error::Sqlite { .. })), "{e:?}");
    Ok(())
}
