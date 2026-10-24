use std::fs;
use std::path::Path;

use super::{is, restore};
use crate::store::tests::{Scratch, Test, chunk, event};
use crate::store::{Error, Mode};

#[test]
fn drop_leaves_a_tombstone_under_the_project() -> Test {
    let s = Scratch::new("tombstone-write")?;
    let mut store = s.open(Mode::Read)?;
    store.append(&event(5, Some("one"), "a"))?;
    let path = chunk(&store, "19700101-7d");
    store.drop(&path)?;
    let file = s.data().join(format!("{path}.dropped"));
    assert!(is(&file) && !is(Path::new("chunks/p/19700101-7d.db")));
    let dir = s.data().join("chunks").join(store.project());
    assert_eq!(file.parent(), Some(dir.as_path()));
    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&file)?)?;
    let field = |at: &str| json.pointer(at).and_then(|v| v.as_str());
    assert_eq!(field("/partition/0"), Some(path.as_str()));
    assert_eq!(field("/refs/0/3"), Some("one"));
    Ok(())
}

/// What restoring a tombstone holding `text` gives.
fn restored(
    s: &Scratch,
    text: &str,
) -> Result<Option<Error>, Box<dyn std::error::Error>> {
    let store = s.open(Mode::Read)?;
    drop(store);
    let file = s.data().join("chunks/x.db.dropped");
    fs::write(&file, text)?;
    let conn = s.sql("catalog.db")?;
    Ok(restore(&conn, &file).err())
}

#[test]
fn an_unreadable_tombstone_fails_the_rebuild() -> Test {
    let s = Scratch::new("tombstone-bad")?;
    for text in ["not json", "{\"partition\": []}", "{\"refs\": []}"] {
        let e = restored(&s, text)?;
        assert!(matches!(e, Some(Error::Io { .. })), "{text}: {e:?}");
    }
    let e = restored(&s, "{\"partition\": [], \"refs\": []}")?;
    assert!(matches!(e, Some(Error::Sqlite { .. })), "{e:?}");
    Ok(())
}

#[test]
fn a_missing_tombstone_is_an_io_error() -> Test {
    let s = Scratch::new("tombstone-missing")?;
    drop(s.open(Mode::Read)?);
    let conn = s.sql("catalog.db")?;
    let e = restore(&conn, &s.data().join("gone.dropped")).err();
    assert!(matches!(e, Some(Error::Io { .. })), "{e:?}");
    Ok(())
}
