use super::{DEFAULTS, Entry, Origin, get, show};
use crate::config::source::{KEYS, TABLES};
use crate::config::tests::{Scratch, Test, put};
use crate::config::{Error as ConfigError, Paths, load_from, paths};

fn entry(entries: &[Entry], key: &str) -> Result<Entry, String> {
    let found = entries.iter().find(|e| e.key == key).cloned();
    found.ok_or_else(|| format!("no {key}"))
}

#[test]
fn paths_name_the_user_and_project_files() -> Test {
    let s = Scratch::new("paths")?;
    let user = Some(s.user_file());
    let want = Paths {
        user,
        project: s.project_file(),
    };
    assert_eq!(paths(Some(&s.home()), &s.project()), want);
    assert_eq!(paths(None, &s.project()).user, None);
    Ok(())
}

#[test]
fn no_files_show_every_leaf_key_as_default() -> Test {
    let s = Scratch::new("showdefaults")?;
    let entries = show(Some(&s.home()), &s.project())?;
    let keys: Vec<&str> = entries.iter().map(|e| e.key).collect();
    let leaves: Vec<&str> = KEYS
        .iter()
        .copied()
        .filter(|k| !TABLES.contains(k))
        .collect();
    assert_eq!(keys, leaves);
    assert!(entries.iter().all(|e| e.origin == Origin::Default));
    assert!(entries.iter().all(|e| e.value.is_some()));
    let window = entry(&entries, "cache.window")?.value;
    assert_eq!(window.as_deref(), Some("\"28d\""));
    Ok(())
}

#[test]
fn data_dir_shows_under_the_home_if_any() -> Test {
    let s = Scratch::new("showdatadir")?;
    let entries = show(Some(&s.home()), &s.project())?;
    let dir = s.home().join(".local/share/chert");
    let want = format!("\"{}\"", dir.display());
    let data_dir = entry(&entries, "storage.data_dir")?.value;
    assert_eq!(data_dir, Some(want));
    let entries = show(None, &s.project())?;
    assert_eq!(entry(&entries, "storage.data_dir")?.value, None);
    Ok(())
}

#[test]
fn each_value_names_the_file_it_came_from() -> Test {
    let s = Scratch::new("showorigin")?;
    let user = "[cache]\nwindow = \"14d\"\n[search]\nalpha = 0.2\n";
    put(&s.user_file(), user)?;
    put(&s.project_file(), "cache.window = \"7d\"\n")?;
    let entries = show(Some(&s.home()), &s.project())?;
    let window = entry(&entries, "cache.window")?;
    assert_eq!(window.value.as_deref(), Some("\"7d\""));
    assert_eq!(window.origin, Origin::Project(s.project_file()));
    let alpha = entry(&entries, "search.alpha")?;
    assert_eq!(alpha.value.as_deref(), Some("0.2"));
    assert_eq!(alpha.origin, Origin::User(s.user_file()));
    Ok(())
}

#[test]
fn show_fails_as_load_does() -> Test {
    let s = Scratch::new("showbad")?;
    put(&s.project_file(), "cache.window = \"4w\"\n")?;
    let e = show(Some(&s.home()), &s.project());
    assert!(matches!(e, Err(ConfigError::Invalid { .. })), "{e:?}");
    Ok(())
}

#[test]
fn get_finds_a_leaf_key_only() -> Test {
    let s = Scratch::new("get")?;
    put(&s.project_file(), "[partition]\nshards = 2\n")?;
    let (home, project) = (Some(s.home()), s.project());
    let shards = get(home.as_deref(), &project, "partition.shards")?;
    let want = Entry {
        key: "partition.shards",
        value: Some("2".to_owned()),
        origin: Origin::Project(s.project_file()),
    };
    assert_eq!(shards, Some(want));
    assert_eq!(get(home.as_deref(), &project, "compression.zstd")?, None);
    assert_eq!(get(home.as_deref(), &project, "cache.size")?, None);
    Ok(())
}

#[test]
fn shown_defaults_load_as_the_defaults() -> Test {
    let s = Scratch::new("showroundtrip")?;
    let lines = DEFAULTS.iter().map(|(k, v)| format!("{k} = {v}\n"));
    let text = lines.collect::<Vec<_>>().concat();
    put(&s.project_file(), &text)?;
    let written = load_from(None, &s.project())?.config;
    let bare = load_from(None, &s.home())?.config;
    assert_eq!(written, bare);
    Ok(())
}
