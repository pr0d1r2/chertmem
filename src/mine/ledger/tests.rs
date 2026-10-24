use std::fs;
use std::path::PathBuf;

use rusqlite::Connection;

use super::{FILE, Key, Ledger, VERSION};
use crate::mine::tests::{Res, Test, reference};
use crate::mine::{Error, Finding, Kind};
use crate::store::{Partition, State, Stats};

/// A scratch data dir, removed on drop.
struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Res<Self> {
        let pid = std::process::id();
        let dir =
            std::env::temp_dir().join(format!("chert-ledger-{pid}-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir)?;
        Ok(Self(dir))
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn partition(path: &str, checksum: &str) -> Partition {
    Partition {
        path: path.to_owned(),
        dims: "project,time".to_owned(),
        project: "p".to_owned(),
        ide: String::new(),
        t0: 0,
        t1: 1,
        shard: None,
        stats: stats(checksum),
        state: State::Warm,
        spec: String::new(),
    }
}

fn stats(checksum: &str) -> Stats {
    Stats {
        rows: 1,
        bytes: 1,
        checksum: checksum.to_owned(),
    }
}

fn key(checksum: &str) -> Key {
    Key::of(&partition("chunks/p/0-7d.db", checksum), "p", 3)
}

fn found() -> Res<Vec<Finding>> {
    let one = |id: &str, kind, ns: &[u64]| -> Res<Finding> {
        let refs: Vec<_> =
            ns.iter().map(|&n| reference(n)).collect::<Res<_>>()?;
        Ok(Finding {
            id: id.to_owned(),
            kind,
            count: refs.len(),
            refs,
            summary: format!("{id} summary"),
        })
    };
    Ok(vec![
        one("00000000000000bb", Kind::HotFile, &[2, 4, 6, 8])?,
        one("00000000000000aa", Kind::Sequence, &[1, 3, 5])?,
    ])
}

#[test]
fn what_is_put_is_got_back_in_order() -> Test {
    let dir = Dir::new("roundtrip")?;
    let mut ledger = Ledger::open(&dir.0)?;
    ledger.put(&key("c1"), &found()?)?;
    assert_eq!(ledger.get(&key("c1"))?, Some(found()?));
    let again = Ledger::open(&dir.0)?;
    assert_eq!(again.get(&key("c1"))?, Some(found()?));
    Ok(())
}

#[test]
fn no_findings_is_kept_too() -> Test {
    let dir = Dir::new("empty")?;
    let mut ledger = Ledger::open(&dir.0)?;
    assert_eq!(ledger.get(&key("c1"))?, None);
    ledger.put(&key("c1"), &[])?;
    assert_eq!(ledger.get(&key("c1"))?, Some(Vec::new()));
    Ok(())
}

#[test]
fn another_checksum_threshold_or_project_is_stale() -> Test {
    let dir = Dir::new("stale")?;
    let mut ledger = Ledger::open(&dir.0)?;
    ledger.put(&key("c1"), &found()?)?;
    assert_eq!(ledger.get(&key("c2"))?, None);
    let higher = Key {
        threshold: 4,
        ..key("c1")
    };
    assert_eq!(ledger.get(&higher)?, None);
    let other = Key {
        project: "q".to_owned(),
        ..key("c1")
    };
    assert_eq!(ledger.get(&other)?, None);
    Ok(())
}

#[test]
fn a_new_put_replaces_the_partitions_findings() -> Test {
    let dir = Dir::new("replace")?;
    let mut ledger = Ledger::open(&dir.0)?;
    ledger.put(&key("c1"), &found()?)?;
    let fewer: Vec<_> = found()?.into_iter().take(1).collect();
    ledger.put(&key("c2"), &fewer)?;
    assert_eq!(ledger.get(&key("c2"))?, Some(fewer));
    assert_eq!(ledger.get(&key("c1"))?, None);
    Ok(())
}

#[test]
fn it_lives_in_its_own_file_under_the_data_dir() -> Test {
    let dir = Dir::new("file")?;
    Ledger::open(&dir.0)?.put(&key("c1"), &[])?;
    assert!(dir.0.join(FILE).is_file());
    let top: Vec<_> = fs::read_dir(&dir.0)?.collect::<Result<_, _>>()?;
    let names: Vec<_> = top.iter().map(fs::DirEntry::file_name).collect();
    assert_eq!(names, ["mine"]);
    Ok(())
}

#[test]
fn a_row_of_an_unknown_kind_is_an_error() -> Test {
    let dir = Dir::new("badkind")?;
    Ledger::open(&dir.0)?.put(&key("c1"), &found()?)?;
    let conn = Connection::open(dir.0.join(FILE))?;
    conn.execute("UPDATE findings SET kind = 'error'", [])?;
    let got = Ledger::open(&dir.0)?.get(&key("c1"));
    assert!(matches!(got, Err(Error::Ledger { .. })), "{got:?}");
    Ok(())
}

/// Token sinks (src/mine T3) changed the signals, so findings kept by
/// the miner before them are mined again.
#[test]
fn the_version_names_the_signals_with_token_sinks() {
    assert!(VERSION.ends_with("/signals-2"), "{VERSION}");
}

#[test]
fn an_unusable_file_is_a_ledger_error() -> Test {
    let dir = Dir::new("unusable")?;
    fs::write(dir.0.join("mine"), "not a dir")?;
    let got = Ledger::open(&dir.0);
    assert!(matches!(got, Err(Error::Ledger { .. })), "{got:?}");
    Ok(())
}
