//! `chert compress` and `chert expand`: the grammar on a file, exit 0;
//! an unreadable file is exit 2 (V2).

use std::fs;

use super::super::tests::{Sandbox, Test};
use crate::grammar::{Intensity, compress, expand};

const PROSE: &str = "I think that the function should basically return.\n";

#[test]
fn compress_prints_the_file_in_caveman_grammar() -> Test {
    let s = Sandbox::new("compress")?;
    let file = s.file("in.txt");
    fs::write(&file, PROSE)?;
    let ran = s.run(&["compress", &file.to_string_lossy()]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert_eq!(ran.out, compress(PROSE, Intensity::Full));
    Ok(())
}

#[test]
fn compress_follows_compression_grammar() -> Test {
    let s = Sandbox::new("compress-off")?;
    s.project_toml("[compression]\ngrammar = \"off\"\n")?;
    let file = s.file("in.txt");
    fs::write(&file, PROSE)?;
    let ran = s.run(&["compress", &file.to_string_lossy()]);
    assert_eq!((ran.code, ran.out.as_str()), (0, PROSE));
    Ok(())
}

#[test]
fn expand_writes_abbreviations_out() -> Test {
    let s = Sandbox::new("expand")?;
    let file = s.file("in.txt");
    let short = compress(PROSE, Intensity::Ultra);
    fs::write(&file, &short)?;
    let ran = s.run(&["expand", &file.to_string_lossy()]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert_eq!(ran.out, expand(&short));
    Ok(())
}

#[test]
fn an_unreadable_file_is_exit_2_naming_it() -> Test {
    let s = Sandbox::new("text-missing")?;
    for verb in ["compress", "expand"] {
        let ran = s.run(&[verb, "no-such-file.txt"]);
        assert_eq!(ran.code, 2, "{verb}: {ran:?}");
        assert!(ran.err.contains("no-such-file.txt"), "{ran:?}");
    }
    Ok(())
}
