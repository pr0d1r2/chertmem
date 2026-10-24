use std::error::Error as StdError;
use std::fs;
use std::time::Duration;

use rusqlite::Connection;

use super::partitions;
use crate::store::schema::{self, CATALOG};
use crate::store::{Filter, State};

type Test = Result<(), Box<dyn StdError>>;

/// A catalog holding partitions `[0,10)` of p, `[10,20)` of q and
/// `[20,30)` with no project dim.
fn catalog(name: &str) -> Result<Connection, Box<dyn StdError>> {
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("chert-prune-{pid}-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let busy = Duration::from_millis(50);
    let conn = schema::open(&dir.join("catalog.db"), busy, CATALOG)?;
    conn.execute_batch(
        "INSERT INTO partitions (path, dims, project, ide, t0, t1, checksum,
           spec, state) VALUES
           ('a', 'project,time', 'p', '', 0, 10, 'c', 's', 'hot'),
           ('b', 'project,time', 'q', '', 10, 20, 'c', 's', 'warm'),
           ('c', 'time', '', '', 20, 30, 'c', 's', 'cold');",
    )?;
    Ok(conn)
}

fn paths(
    conn: &Connection,
    filter: &Filter,
) -> Result<Vec<String>, Box<dyn StdError>> {
    Ok(partitions(conn, filter)?
        .into_iter()
        .map(|p| p.path)
        .collect())
}

fn window(since: Option<u64>, until: Option<u64>) -> Filter {
    Filter {
        project: None,
        since,
        until,
    }
}

#[test]
fn bounds_are_half_open() -> Test {
    let c = catalog("bounds")?;
    assert_eq!(paths(&c, &Filter::default())?, ["a", "b", "c"]);
    assert_eq!(paths(&c, &window(Some(10), None))?, ["b", "c"]);
    assert_eq!(paths(&c, &window(None, Some(10)))?, ["a"]);
    assert_eq!(paths(&c, &window(Some(9), Some(11)))?, ["a", "b"]);
    assert!(paths(&c, &window(Some(30), None))?.is_empty());
    Ok(())
}

#[test]
fn a_project_keeps_its_own_and_dimless_partitions() -> Test {
    let c = catalog("project")?;
    let p = Filter {
        project: Some("p".into()),
        ..Filter::default()
    };
    assert_eq!(paths(&c, &p)?, ["a", "c"]);
    Ok(())
}

#[test]
fn every_catalog_column_comes_back() -> Test {
    let c = catalog("columns")?;
    let all = partitions(&c, &Filter::default())?;
    let states: Vec<State> = all.iter().map(|p| p.state).collect();
    assert_eq!(states, [State::Hot, State::Warm, State::Cold]);
    let b = all.get(1).ok_or("no b")?;
    assert_eq!((b.dims.as_str(), b.project.as_str()), ("project,time", "q"));
    assert_eq!((b.t0, b.t1, b.shard, b.stats.rows), (10, 20, None, 0));
    Ok(())
}
