//! The config file as bytes and JSON, its writes and backup.

use std::fs;
use std::path::Path;

use serde_json::json;

use super::{back_up, backup, bytes, load, parse, remove, render, write};
use crate::install::Error;
use crate::install::tests::{Scratch, Test};

#[test]
fn no_file_loads_as_an_empty_object_without_bytes() -> Test {
    let s = Scratch::new("settings-absent")?;
    let got = load(&s.settings)?;
    assert_eq!((got.bytes, got.value), (None, json!({})));
    Ok(())
}

#[test]
fn blank_text_parses_as_an_empty_object() -> Test {
    assert_eq!(parse(Path::new("x"), b" \n\t")?, json!({}));
    let got = parse(Path::new("x"), b"{");
    assert!(matches!(got, Err(Error::Json { .. })), "{got:?}");
    Ok(())
}

#[test]
fn render_indents_by_2_and_ends_with_a_newline() {
    let got = render(&json!({"b": 1, "a": [true]}));
    let want = "{\n  \"a\": [\n    true\n  ],\n  \"b\": 1\n}\n";
    assert_eq!(String::from_utf8_lossy(&got), want);
}

#[test]
fn the_backup_sits_beside_the_file() {
    let got = backup(Path::new("dir/settings.json"));
    assert_eq!(got, Path::new("dir/settings.json.chert.bak"));
}

#[test]
fn back_up_keeps_the_first_bytes_only() -> Test {
    let s = Scratch::new("settings-backup")?;
    let path = s.settings.clone();
    back_up(&path, b"first")?;
    back_up(&path, b"second")?;
    assert_eq!(fs::read(backup(&path))?, b"first");
    Ok(())
}

#[test]
fn write_makes_the_dir_and_leaves_no_temp_file() -> Test {
    let s = Scratch::new("settings-write")?;
    let path = s.settings.clone();
    write(&path, b"{}\n")?;
    assert_eq!(fs::read(&path)?, b"{}\n");
    let names: Vec<_> = fs::read_dir(path.parent().ok_or("no parent")?)?
        .filter_map(Result::ok)
        .map(|e| e.file_name())
        .collect();
    assert_eq!(names, ["settings.json"]);
    Ok(())
}

#[test]
fn io_failures_name_the_path() -> Test {
    let s = Scratch::new("settings-io")?;
    let got = bytes(&s.dir);
    assert!(matches!(got, Err(Error::Io { .. })), "{got:?}");
    let blocked = s.dir.join("file");
    fs::write(&blocked, "x")?;
    let got = write(&blocked.join("settings.json"), b"{}");
    assert!(matches!(got, Err(Error::Io { .. })), "{got:?}");
    let got = remove(&s.dir);
    assert!(matches!(got, Err(Error::Io { .. })), "{got:?}");
    remove(&s.dir.join("absent"))?;
    Ok(())
}
