use std::error::Error as StdError;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use super::{Chunks, insert};
use crate::store::Event;
use crate::store::write::Row;

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

#[test]
fn get_fails_where_no_dir_can_be_made() -> Test {
    let dir = root("blocked")?;
    fs::write(dir.join("chunks"), "a file")?;
    let mut chunks = Chunks::new(dir, Duration::from_millis(50));
    assert!(chunks.get("chunks/p/a.db").is_err());
    Ok(())
}
