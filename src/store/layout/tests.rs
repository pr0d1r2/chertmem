use std::error::Error as StdError;
use std::path::Path;

use super::{FNV_OFFSET, Key, Plan, fnv1a, project_id, slug};
use crate::config::{Dimension, Partition, Span, Unit};
use crate::store::Error;

type Test = Result<(), Box<dyn StdError>>;

fn plan(by: &[Dimension], days: u32, shards: u32) -> Plan {
    Plan::new(&Partition {
        by: by.to_vec(),
        time_bucket: Span {
            amount: days,
            unit: Unit::Days,
        },
        shards,
    })
}

const DEFAULT: &[Dimension] = &[Dimension::Project, Dimension::Time];

fn key(ts: u64, id: &str) -> Key<'_> {
    Key {
        project: "proj-1",
        ide: "Claude Code",
        ts,
        id,
    }
}

#[test]
fn fnv1a_matches_the_reference_vectors() {
    assert_eq!(fnv1a(FNV_OFFSET, b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a(FNV_OFFSET, b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(
        fnv1a(fnv1a(FNV_OFFSET, b"ab"), b"c"),
        fnv1a(FNV_OFFSET, b"abc")
    );
}

#[test]
fn project_id_is_stable_and_names_the_dir() {
    let a = project_id(Path::new("/no/such/dir/My Repo"));
    assert_eq!(a, project_id(Path::new("/no/such/dir/My Repo")));
    assert!(a.starts_with("my-repo-") && a.len() == "my-repo-".len() + 16);
    assert_ne!(a, project_id(Path::new("/other/My Repo")));
    assert!(project_id(Path::new("/")).starts_with("root-"));
}

#[test]
fn slug_keeps_safe_ascii_only() {
    assert_eq!(slug("Claude Code/../x"), "claude-code----x");
    assert_eq!(slug(&"a".repeat(40)).len(), 32);
}

#[test]
fn default_plan_puts_a_row_under_its_project_and_bucket() -> Test {
    let p = plan(DEFAULT, 7, 1).place(key(0, "01ID"))?;
    assert_eq!(p.path, "chunks/proj-1/19700101-7d.db");
    assert_eq!((p.project.as_str(), p.ide.as_str()), ("proj-1", ""));
    assert_eq!((p.t0, p.t1, p.shard), (0, 7 * 86_400_000, None));
    Ok(())
}

#[test]
fn dims_shape_the_dir() -> Test {
    let by = [Dimension::Time, Dimension::Ide, Dimension::Project];
    let p = plan(&by, 1, 1).place(key(0, "01ID"))?;
    assert_eq!(p.path, "chunks/claude-code+proj-1/19700101-1d.db");
    let time_only = plan(&[Dimension::Time], 1, 1).place(key(0, "01ID"))?;
    assert_eq!(time_only.path, "chunks/_/19700101-1d.db");
    assert_eq!(time_only.project, "");
    let blank = Key {
        ide: "",
        ..key(0, "01ID")
    };
    let by = [Dimension::Ide, Dimension::Time];
    assert_eq!(plan(&by, 1, 1).place(blank)?.ide, "_");
    Ok(())
}

#[test]
fn shards_split_by_project_and_id() -> Test {
    let four = plan(DEFAULT, 7, 4);
    let mut seen = Vec::new();
    for id in ["A", "B", "C", "D", "E", "F", "G", "H"] {
        let p = four.place(key(0, id))?;
        let shard = p.shard.ok_or("no shard")?;
        assert!(shard < 4);
        assert!(p.path.ends_with(&format!("19700101-7d.{shard}.db")));
        assert_eq!(four.place(key(0, id))?.shard, Some(shard));
        seen.push(shard);
    }
    seen.dedup();
    assert!(seen.len() > 1, "{seen:?}");
    Ok(())
}

#[test]
fn spec_records_the_plan() {
    let p = plan(DEFAULT, 7, 2);
    assert_eq!(p.dims(), "project,time");
    assert_eq!(p.spec(), "by=project,time;time_bucket=7d;shards=2");
}

#[test]
fn a_timestamp_past_48_bits_has_no_place() {
    let e = plan(DEFAULT, 7, 1).place(key(1 << 48, "01ID")).err();
    assert_eq!(e, Some(Error::BadTime { ts: 1 << 48 }));
}
