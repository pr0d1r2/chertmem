use std::error::Error as StdError;
use std::fs;
use std::time::Duration;

use rusqlite::Connection;

use super::{checksum, ensure, existing, record, taken};
use crate::store::layout::Place;
use crate::store::schema::{self, CATALOG};
use crate::store::write::Row;
use crate::store::{Error, Event};

type Test = Result<(), Box<dyn StdError>>;

fn catalog(name: &str) -> Result<Connection, Box<dyn StdError>> {
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("chert-cat-{pid}-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let busy = Duration::from_millis(50);
    Ok(schema::open(&dir.join("catalog.db"), busy, CATALOG)?)
}

fn place(path: &str) -> Place {
    Place {
        path: path.to_owned(),
        project: "p".to_owned(),
        ide: String::new(),
        t0: 0,
        t1: 10,
        shard: None,
    }
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
fn ensure_makes_one_row_per_path() -> Test {
    let c = catalog("ensure")?;
    let a = ensure(&c, &place("chunks/p/a.db"), "project,time", "spec")?;
    let again = ensure(&c, &place("chunks/p/a.db"), "project,time", "x")?;
    let b = ensure(&c, &place("chunks/p/b.db"), "project,time", "spec")?;
    assert_eq!(a, again);
    assert_ne!(a.id, b.id);
    let spec: String = c.query_row(
        "SELECT spec FROM partitions WHERE id = ?1",
        [a.id],
        |r| r.get(0),
    )?;
    assert_eq!(spec, "spec");
    Ok(())
}

#[test]
fn ensure_refuses_a_sealed_partition() -> Test {
    let c = catalog("sealed")?;
    ensure(&c, &place("chunks/p/a.db"), "d", "s")?;
    c.execute("UPDATE partitions SET state = 'warm'", [])?;
    let e = ensure(&c, &place("chunks/p/a.db"), "d", "s").err();
    let path = "chunks/p/a.db".to_owned();
    assert_eq!(e, Some(Error::ReadOnly { path }));
    Ok(())
}

#[test]
fn record_issues_the_ref_and_grows_the_partition() -> Test {
    let c = catalog("record")?;
    let slot = ensure(&c, &place("chunks/p/a.db"), "d", "s")?;
    assert!(!taken(&c, "p", "01AB")?);
    record(&c, &slot, &row("01ABCDEF", Some("tool-1")), "01AB")?;
    assert!(taken(&c, "p", "01AB")? && !taken(&c, "q", "01AB")?);
    let by_id = existing(&c, &row("01ABCDEF", None))?;
    let by_identity = existing(&c, &row("OTHER", Some("tool-1")))?;
    let fresh = existing(&c, &row("OTHER", None))?;
    assert_eq!(
        (by_id, by_identity),
        (Some("01AB".into()), Some("01AB".into()))
    );
    assert_eq!(fresh, None);
    Ok(())
}

#[test]
fn record_grows_rows_and_bytes() -> Test {
    let c = catalog("grow")?;
    let slot = ensure(&c, &place("chunks/p/a.db"), "d", "s")?;
    record(&c, &slot, &row("01ABCDEF", None), "01AB")?;
    let (rows, bytes): (u64, u64) =
        c.query_row("SELECT rows, bytes FROM partitions", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?;
    assert_eq!((rows, bytes), (1, 4));
    Ok(())
}

#[test]
fn checksum_folds_rows_in_order() {
    let start = format!("{:016x}", super::FNV_OFFSET);
    let a_then_b = checksum(&checksum(&start, "A", "x"), "B", "y");
    let b_then_a = checksum(&checksum(&start, "B", "y"), "A", "x");
    assert_ne!(a_then_b, b_then_a);
    assert_eq!(a_then_b, checksum(&checksum(&start, "A", "x"), "B", "y"));
    assert_eq!(a_then_b.len(), 16);
    assert_ne!(checksum(&start, "A", "xy"), checksum(&start, "Ax", "y"));
}
