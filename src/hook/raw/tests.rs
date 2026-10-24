use std::fs;
use std::path::PathBuf;

use super::keep;
use crate::hook::tests::{NOW, Res, Scratch, Test};
use crate::hook::{Ctx, Ide, Kind};
use crate::store::Error;

/// Keep `stdin` raw through the store, as the hook does for `scratch`.
fn keep_in(scratch: &Scratch, stdin: &[u8]) -> Result<PathBuf, Error> {
    let root = scratch.root();
    let ctx = Ctx {
        config: &scratch.config,
        root: &root,
        now: NOW,
        ide: Ide::ClaudeCode,
    };
    keep(&ctx, stdin)
}

/// The raw payloads kept under `spool/unreadable/`, by name.
fn kept(scratch: &Scratch) -> Res<Vec<Vec<u8>>> {
    let dir = scratch.data().join("spool/unreadable");
    let mut paths: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(entries) => entries
            .map(|e| e.map(|e| e.path()))
            .collect::<Result<_, _>>()?,
        Err(_) => Vec::new(),
    };
    paths.sort();
    Ok(paths.iter().map(fs::read).collect::<Result<_, _>>()?)
}

#[test]
fn a_malformed_payload_is_kept_raw_and_exits_0() -> Test {
    let bad: [&[u8]; 4] =
        [b"not json", b"[1]", br#"{"prompt":"x"}"#, b"\xff\xfe"];
    for (n, text) in bad.into_iter().enumerate() {
        let scratch = Scratch::new(&format!("malformed-{n}"))?;
        let out = scratch.run(Kind::UserPromptSubmit, text);
        assert_eq!((out.exit, out.stdout), (0, None), "{text:?}");
        assert!(out.note.is_some_and(|n| n.contains("unreadable")));
        assert_eq!(kept(&scratch)?, [text.to_vec()], "{text:?}");
        assert!(scratch.observations()?.is_empty());
    }
    Ok(())
}

#[test]
fn a_kept_payload_loses_its_private_spans() -> Test {
    let scratch = Scratch::new("rawprivate")?;
    scratch.run(Kind::Stop, b"{broken <private>pw</private> json");
    assert_eq!(kept(&scratch)?, [b"{broken  json".to_vec()]);
    Ok(())
}

#[test]
fn a_reading_open_leaves_a_kept_payload_in_place() -> Test {
    let scratch = Scratch::new("rawstays")?;
    scratch.run(Kind::Stop, b"nope");
    assert!(scratch.observations()?.is_empty());
    assert!(scratch.observations()?.is_empty());
    assert_eq!(kept(&scratch)?, [b"nope".to_vec()]);
    Ok(())
}

#[test]
fn a_failing_store_keeps_the_payload_raw() -> Test {
    let scratch = Scratch::new("rawfail")?;
    fs::create_dir_all(scratch.data().join("catalog.db"))?;
    let text = br#"{"session_id":"s","prompt":"hi"}"#;
    let out = scratch.run(Kind::UserPromptSubmit, text);
    assert_eq!((out.exit, out.stdout), (0, None));
    assert!(out.note.is_some_and(|n| n.contains("kept raw")));
    assert_eq!(kept(&scratch)?, [text.to_vec()]);
    Ok(())
}

#[test]
fn without_a_data_dir_the_note_says_lost_and_exit_is_0() -> Test {
    let mut scratch = Scratch::new("rawnodir")?;
    scratch.config.storage.data_dir = None;
    let out = scratch.run(Kind::UserPromptSubmit, b"not json");
    assert_eq!((out.exit, out.stdout), (0, None));
    assert!(out.note.is_some_and(|n| n.contains("lost")));
    Ok(())
}

#[test]
fn an_unwritable_spool_says_lost_and_exit_is_0() -> Test {
    let scratch = Scratch::new("rawunwritable")?;
    fs::create_dir_all(scratch.data())?;
    fs::write(scratch.data().join("spool"), "a file, not a dir")?;
    let out = scratch.run(Kind::UserPromptSubmit, b"not json");
    assert_eq!(out.exit, 0);
    assert!(out.note.is_some_and(|n| n.contains("lost")));
    Ok(())
}

#[test]
fn a_kept_payload_lands_where_the_store_spools_it() -> Test {
    let scratch = Scratch::new("rawviastore")?;
    let file = keep_in(&scratch, b"a <private>pw</private>b")?;
    let dir = scratch.data().join("spool/unreadable");
    assert_eq!(file.parent(), Some(dir.as_path()));
    let stem = file.file_stem().and_then(|x| x.to_str()).ok_or("name")?;
    assert!(ulid::Ulid::from_string(stem).is_ok(), "{stem}");
    assert_eq!(fs::read(&file)?, b"a b");
    Ok(())
}

#[test]
fn keeping_fails_with_the_store_s_own_errors() -> Test {
    let mut scratch = Scratch::new("rawstoreerr")?;
    fs::create_dir_all(scratch.data())?;
    fs::write(scratch.data().join("spool"), "a file, not a dir")?;
    let blocked = keep_in(&scratch, b"x").err();
    assert!(matches!(blocked, Some(Error::Io { .. })), "{blocked:?}");
    scratch.config.storage.data_dir = None;
    let none = keep_in(&scratch, b"x").err();
    assert!(matches!(none, Some(Error::NoDataDir)), "{none:?}");
    Ok(())
}
