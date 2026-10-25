//! `status` and `doctor` on temp configs and temp stores (src/install
//! V3).

use std::fs;
use std::path::PathBuf;

use super::{Drift, Finding, Report, Wire, WireState, doctor, status};
use crate::config::{Config, load_from};
use crate::hook::Ide;
use crate::install::tests::{Res, Scratch, Test, target};
use crate::install::{Error, install};
use crate::store::{self, Event, Filter, Mode, Store};

/// Milliseconds of 2026-10-24.
const NOW: u64 = 1_792_800_000_000;

/// A scratch home with a project dir and a data dir not made yet.
struct Home {
    scratch: Scratch,
    config: Config,
}

impl Home {
    fn new(name: &str) -> Res<Self> {
        let scratch = Scratch::new(&format!("doctor-{name}"))?;
        fs::create_dir_all(scratch.dir.join("project"))?;
        let mut config = load_from(None, &scratch.dir.join("project"))?.config;
        config.storage.data_dir = Some(scratch.dir.join("data"));
        Ok(Self { scratch, config })
    }

    fn root(&self) -> PathBuf {
        self.scratch.dir.join("project")
    }

    fn data(&self) -> PathBuf {
        self.scratch.dir.join("data")
    }

    /// The settings file, wired.
    fn wired(&self) -> Res<PathBuf> {
        let path = self.scratch.settings();
        install(&target(&path), Ide::ClaudeCode)?;
        Ok(path)
    }

    /// 1 stored event; the catalog path of its partition.
    fn append(&self) -> Res<String> {
        let mut store = Store::open(&self.config, &self.root(), Mode::Write)?;
        store.append(&Event {
            ts: NOW,
            session: "s1".to_owned(),
            kind: "stop".to_owned(),
            ide: "claude-code".to_owned(),
            identity: None,
            body: "fixed the auth bug".to_owned(),
        })?;
        let store = Store::open(&self.config, &self.root(), Mode::Read)?;
        let all = store.partitions(&Filter::default())?;
        Ok(all.first().ok_or("no partition")?.path.clone())
    }

    fn doctor(&self) -> Res<Report> {
        let path = self.scratch.settings();
        Ok(doctor(
            &target(&path),
            Ide::ClaudeCode,
            &self.config,
            &self.root(),
        )?)
    }
}

#[test]
fn status_reads_each_entry_before_and_after_install() -> Test {
    let home = Home::new("status")?;
    let path = home.scratch.settings();
    let before = status(&target(&path), Ide::ClaudeCode)?;
    assert_eq!(before.settings, path);
    assert_eq!(before.wires.len(), 6);
    assert!(!before.wired());
    home.wired()?;
    assert!(status(&target(&path), Ide::ClaudeCode)?.wired());
    Ok(())
}

#[test]
fn a_wired_config_without_a_data_dir_is_healthy() -> Test {
    let home = Home::new("healthy")?;
    home.wired()?;
    let report = home.doctor()?;
    assert_eq!((report.exit(), report.findings), (0, vec![]));
    assert!(!home.data().exists(), "doctor made the data dir");
    Ok(())
}

#[test]
fn doctor_exits_1_naming_each_broken_wiring() -> Test {
    let home = Home::new("unwired")?;
    let report = home.doctor()?;
    assert_eq!(report.exit(), 1);
    let lines: Vec<String> =
        report.findings.iter().map(ToString::to_string).collect();
    assert_eq!(lines.len(), 6);
    let stop = "wiring hook Stop: missing (run chert install)";
    assert!(lines.iter().any(|l| l == stop), "{lines:?}");
    let server = "wiring mcp server chert: missing (run chert install)";
    assert!(lines.iter().any(|l| l == server), "{lines:?}");
    Ok(())
}

#[test]
fn a_store_in_step_with_its_files_is_healthy() -> Test {
    let home = Home::new("clean")?;
    home.wired()?;
    home.append()?;
    assert_eq!(home.doctor()?.findings, vec![]);
    Ok(())
}

fn drift(path: &str, drift: Drift) -> Finding {
    Finding::Drift {
        path: path.to_owned(),
        drift,
    }
}

#[test]
fn a_lost_catalog_is_drift_once_then_rebuilt() -> Test {
    let home = Home::new("lost")?;
    home.wired()?;
    let path = home.append()?;
    for name in ["catalog.db", "catalog.db-wal", "catalog.db-shm"] {
        let _ = fs::remove_file(home.data().join(name));
    }
    let report = home.doctor()?;
    assert_eq!(report.findings, vec![drift(&path, Drift::Unlisted)]);
    assert_eq!(report.exit(), 1);
    assert_eq!(home.doctor()?.exit(), 0);
    Ok(())
}

#[test]
fn a_gone_partition_file_is_drift() -> Test {
    let home = Home::new("gone")?;
    home.wired()?;
    let path = home.append()?;
    fs::remove_file(home.data().join(&path))?;
    let report = home.doctor()?;
    assert_eq!(report.findings, vec![drift(&path, Drift::Gone)]);
    Ok(())
}

#[test]
fn a_catalog_row_unlike_its_file_is_drift() -> Test {
    let home = Home::new("differs")?;
    home.wired()?;
    let path = home.append()?;
    let catalog = rusqlite::Connection::open(home.data().join("catalog.db"))?;
    catalog.execute("UPDATE partitions SET rows = 99", [])?;
    drop(catalog);
    let report = home.doctor()?;
    assert_eq!(report.findings, vec![drift(&path, Drift::Differs)]);
    Ok(())
}

/// A wired home with 1 unreadable hook payload and 1 sealed spool file
/// beside a temp file.
fn spooled(name: &str) -> Res<Home> {
    let home = Home::new(name)?;
    home.wired()?;
    let store = Store::open(&home.config, &home.root(), Mode::Write)?;
    store.spool_raw(b"{ not json")?;
    let sealed = home.data().join("spool/sealed");
    fs::create_dir_all(&sealed)?;
    fs::write(sealed.join("01J.jsonl"), "{}\n")?;
    fs::write(sealed.join(".01K.tmp"), "")?;
    Ok(home)
}

#[test]
fn spool_files_no_replay_takes_are_stale() -> Test {
    let home = spooled("spool")?;
    let report = home.doctor()?;
    let spool = |dir: &str| Finding::Spool {
        dir: home.data().join(dir),
        files: 1,
    };
    let want = vec![spool("spool/sealed"), spool("spool/unreadable")];
    assert_eq!(report.findings, want);
    Ok(())
}

#[test]
fn a_stale_spool_reads_as_its_count_and_dir() -> Test {
    let home = spooled("spool-line")?;
    let line = home.doctor()?.findings.last().map(ToString::to_string);
    let dir = home.data().join("spool/unreadable");
    let want =
        format!("stale spool: 1 file(s) in {} never replayed", dir.display());
    assert_eq!(line, Some(want));
    Ok(())
}

#[test]
fn no_data_dir_setting_is_a_store_error() -> Test {
    let mut home = Home::new("no-data-dir")?;
    home.config.storage.data_dir = None;
    let path = home.scratch.settings();
    let got =
        doctor(&target(&path), Ide::ClaudeCode, &home.config, &home.root());
    assert_eq!(got, Err(Error::Store(store::Error::NoDataDir)));
    Ok(())
}

#[test]
fn an_unreadable_spool_dir_is_an_error() -> Test {
    let home = Home::new("spool-file")?;
    fs::create_dir_all(home.data())?;
    fs::write(home.data().join("spool"), "not a dir")?;
    let got = super::count(&home.data().join("spool"));
    assert!(matches!(got, Err(Error::Io { .. })), "{got:?}");
    Ok(())
}

fn wire_line(state: WireState) -> String {
    let name = "hook Stop".to_owned();
    Finding::Wiring(Wire { name, state }).to_string()
}

#[test]
fn a_wiring_finding_reads_as_its_state() {
    assert_eq!(wire_line(WireState::Ok), "wiring hook Stop: ok");
    let other = wire_line(WireState::Other("old stop".to_owned()));
    let want = "wiring hook Stop: runs another chert: old stop";
    assert_eq!(other, format!("{want} (run chert install)"));
}

#[test]
fn a_drift_finding_reads_as_what_was_wrong() {
    let line = |d| drift("c.db", d).to_string();
    let gone = "catalog drift: c.db: catalog row, file gone; marked dropped";
    assert_eq!(line(Drift::Gone), gone);
    let unlisted = "file missing from the catalog; listed again";
    assert_eq!(
        line(Drift::Unlisted),
        format!("catalog drift: c.db: {unlisted}")
    );
    let differs = "catalog drift: c.db: catalog row unlike the file; rebuilt";
    assert_eq!(line(Drift::Differs), differs);
}
