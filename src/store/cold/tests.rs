use std::error::Error as StdError;
use std::fs;
use std::path::{Path, PathBuf};

use super::{dict, freeze, remove, thaw, thawed, zst};
use crate::config::Zstd;

type Test = Result<(), Box<dyn StdError>>;

const PATH: &str = "chunks/p/19700101-7d.db";

fn root(name: &str) -> Result<PathBuf, Box<dyn StdError>> {
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("chert-cold-{pid}-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("chunks/p"))?;
    Ok(dir)
}

/// 400 KiB of rows alike enough for a dictionary to learn from.
fn source(root: &Path) -> Result<(PathBuf, Vec<u8>), Box<dyn StdError>> {
    let rows = (0..8000).map(|i| {
        format!(
            "{{\"tool\":\"Bash\",\"n\":{i},\"cmd\":\"cargo test {}\"}}\n",
            i % 37
        )
    });
    let bytes = rows.collect::<String>().into_bytes();
    let src = root.join("vacuum.tmp");
    fs::write(&src, &bytes)?;
    Ok((src, bytes))
}

const fn zstd(level: i32, dictionary: bool) -> Zstd {
    Zstd { level, dictionary }
}

#[test]
fn freeze_then_thaw_gives_the_bytes_back_with_a_dictionary() -> Test {
    let root = root("dict")?;
    let (src, bytes) = source(&root)?;
    freeze(&src, &root, PATH, zstd(3, true))?;
    assert!(dict(&root, PATH).is_file());
    let small = fs::metadata(zst(&root, PATH))?.len();
    assert!(small < u64::try_from(bytes.len())? / 4, "{small}");
    let out = thaw(&root, PATH)?;
    assert_eq!(out, thawed(&root, PATH));
    assert_eq!(fs::read(&out)?, bytes);
    fs::write(&out, "kept")?;
    assert_eq!(fs::read(thaw(&root, PATH)?)?, b"kept");
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn freeze_without_dictionary_and_remove_every_file() -> Test {
    let root = root("plain")?;
    let (src, bytes) = source(&root)?;
    freeze(&src, &root, PATH, zstd(1, false))?;
    assert!(!dict(&root, PATH).exists());
    assert_eq!(fs::read(thaw(&root, PATH)?)?, bytes);
    fs::write(root.join(PATH), "hot file")?;
    remove(&root, PATH)?;
    remove(&root, PATH)?;
    for gone in [zst(&root, PATH), thawed(&root, PATH), root.join(PATH)] {
        assert!(!gone.exists(), "{}", gone.display());
    }
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn a_tiny_file_compresses_without_a_dictionary() -> Test {
    let root = root("tiny")?;
    let src = root.join("vacuum.tmp");
    fs::write(&src, "too small to train on")?;
    freeze(&src, &root, PATH, zstd(3, true))?;
    assert!(!dict(&root, PATH).exists());
    assert_eq!(fs::read(thaw(&root, PATH)?)?, b"too small to train on");
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn thawing_nothing_is_an_io_error() -> Test {
    let root = root("missing")?;
    let e = thaw(&root, PATH).err();
    assert!(matches!(e, Some(crate::store::Error::Io { .. })), "{e:?}");
    fs::remove_dir_all(&root)?;
    Ok(())
}
