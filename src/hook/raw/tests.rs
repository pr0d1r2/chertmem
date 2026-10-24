use std::fs;
use std::path::PathBuf;

use crate::hook::Kind;
use crate::hook::tests::{Res, Scratch, Test};

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
