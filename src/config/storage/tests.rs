use crate::config::load_from;
use crate::config::tests::{Scratch, Test};

#[test]
fn data_dir_defaults_under_home() -> Test {
    let s = Scratch::new("datadir")?;
    let dir = s.load()?.config.storage.data_dir;
    assert_eq!(dir, Some(s.home().join(".local/share/chert")));
    let none = load_from(None, &s.project())?.config.storage.data_dir;
    assert_eq!(none, None);
    Ok(())
}
