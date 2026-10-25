use std::error::Error as StdError;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use super::{Chunks, SCAN, insert, scan};
use crate::store::write::Row;
use crate::store::{Event, Filter};

type Test = Result<(), Box<dyn StdError>>;

fn root(name: &str) -> Result<PathBuf, Box<dyn StdError>> {
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("chert-part-{pid}-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn row(id: &str, identity: Option<&str>) -> Row {
    Row {
        id: id.to_owned(),
        project: "p".to_owned(),
        event: Event {
            ts: 1,
            session: "s".into(),
            kind: "note".into(),
            ide: "claude".into(),
            identity: identity.map(str::to_owned),
            body: "body".into(),
        },
    }
}

#[test]
fn get_makes_the_file_and_its_dirs_once() -> Test {
    let dir = root("get")?;
    let mut chunks = Chunks::new(dir.clone(), Duration::from_millis(50));
    chunks.get("chunks/p/a.db")?;
    chunks.get("chunks/p/a.db")?;
    assert!(dir.join("chunks/p/a.db").is_file());
    Ok(())
}

#[test]
fn insert_gives_back_the_ref_held() -> Test {
    let mut chunks = Chunks::new(root("insert")?, Duration::from_millis(50));
    let conn = chunks.get("a.db")?;
    assert_eq!(insert(conn, &row("01AB", Some("t")), "01A")?, "01A");
    // A row the catalog never heard of keeps its first ref.
    assert_eq!(insert(conn, &row("01AB", None), "01AB")?, "01A");
    assert_eq!(insert(conn, &row("01CD", Some("t")), "01C")?, "01A");
    assert_eq!(insert(conn, &row("01EF", None), "01E")?, "01E");
    Ok(())
}

/// What `EXPLAIN QUERY PLAN` says of [`SCAN`] with these bounds.
fn plan(
    conn: &rusqlite::Connection,
    since: Option<u64>,
    until: Option<u64>,
) -> Result<String, Box<dyn StdError>> {
    let mut stmt = conn.prepare(&format!("EXPLAIN QUERY PLAN {SCAN}"))?;
    let args = rusqlite::params![Some("p"), since, until];
    let rows = stmt.query_map(args, |r| r.get::<_, String>(3))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?.join("; "))
}

/// A scan that searches the time index, never the whole table.
const INDEXED: &str = "SEARCH observations USING INDEX observations_ts";

#[test]
fn a_bounded_scan_searches_observations_ts() -> Test {
    let mut chunks = Chunks::new(root("plan")?, Duration::from_millis(50));
    let conn = chunks.get("a.db")?;
    for (since, until) in [(Some(5), Some(6)), (Some(5), None), (None, Some(6))]
    {
        let found = plan(conn, since, until)?;
        let searched = found.contains(INDEXED);
        assert!(searched, "{since:?} {until:?}: {found}");
    }
    Ok(())
}

/// The ids of project `p` a scan of `[since, until)` gives, in order.
fn ids(
    conn: &rusqlite::Connection,
    since: Option<u64>,
    until: Option<u64>,
) -> Result<Vec<String>, Box<dyn StdError>> {
    let f = Filter {
        project: Some("p".to_owned()),
        since,
        until,
    };
    Ok(scan(conn, &f)?.into_iter().map(|o| o.id).collect())
}

#[test]
fn a_bounded_scan_keeps_its_rows_and_order() -> Test {
    let mut chunks = Chunks::new(root("bounds")?, Duration::from_millis(50));
    let conn = chunks.get("a.db")?;
    for (id, ts) in [("01C", 7), ("01A", 5), ("01B", 6), ("01D", 5)] {
        let mut r = row(id, None);
        r.event.ts = ts;
        insert(conn, &r, id)?;
    }
    assert_eq!(ids(conn, None, None)?, ["01A", "01D", "01B", "01C"]);
    assert_eq!(ids(conn, Some(6), None)?, ["01B", "01C"]);
    assert_eq!(ids(conn, None, Some(6))?, ["01A", "01D"]);
    assert_eq!(ids(conn, Some(5), Some(6))?, ["01A", "01D"]);
    Ok(())
}

#[test]
fn get_fails_where_no_dir_can_be_made() -> Test {
    let dir = root("blocked")?;
    fs::write(dir.join("chunks"), "a file")?;
    let mut chunks = Chunks::new(dir, Duration::from_millis(50));
    assert!(chunks.get("chunks/p/a.db").is_err());
    Ok(())
}
