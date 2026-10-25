use super::{BASELINE, INDEX};
use crate::cli::run;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

type Res = Result<(), Box<dyn Error>>;

fn scratch(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let dir = std::env::temp_dir()
        .join(format!("chertmem-dev-{}-bench-{name}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(dir.join("benches"))?;
    fs::create_dir_all(dir.join("docs"))?;
    Ok(dir)
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

/// A criterion run of 1 bench whose calls all took `ns`.
fn plant(root: &Path, suite: &str, fixture: &str, ns: u32) -> Res {
    let crit = root.join("target/criterion");
    let new = crit.join(suite).join(fixture).join("new");
    fs::create_dir_all(&new)?;
    let estimates = format!(r#"{{"median":{{"point_estimate":{ns}.0}}}}"#);
    fs::write(new.join("estimates.json"), estimates)?;
    let sample = format!(
        r#"{{"iters":[1.0,2.0],"times":[{ns}.0,{}.0]}}"#,
        u64::from(ns).saturating_mul(2)
    );
    fs::write(new.join("sample.json"), sample)?;
    fs::write(crit.join(INDEX), "get_observations\tget_observations\n")?;
    Ok(())
}

/// A repository with 1 bench recorded at 1 ms.
fn recorded(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let root = scratch(name)?;
    plant(&root, "get_observations", "1k-p52", 1_000_000)?;
    assert_eq!(run(&args(&["bench-record"]), &root), 0);
    Ok(root)
}

#[test]
fn bench_record_writes_a_row_of_this_machine() -> Res {
    let root = recorded("record")?;
    let text = fs::read_to_string(root.join(BASELINE))?;
    let rows = crate::baseline::parse(&text)?;
    let host = crate::host::this().ok_or("no host")?;
    let r = rows.first().ok_or("no row")?;
    assert_eq!(
        (r.machine.as_str(), r.os.as_str()),
        (host.machine.as_str(), host.os.as_str())
    );
    assert_eq!((rows.len(), r.median_ms), (1, 1.0));
    Ok(())
}

#[test]
fn bench_record_without_a_run_is_unreadable_input() -> Res {
    assert_eq!(run(&args(&["bench-record"]), &scratch("no-run")?), 2);
    let root = scratch("bad-baseline")?;
    plant(&root, "get_observations", "1k-p52", 5)?;
    fs::write(root.join(BASELINE), "[[row]]\n")?;
    assert_eq!(run(&args(&["bench-record"]), &root), 2);
    Ok(())
}
