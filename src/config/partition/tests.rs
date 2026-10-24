use std::path::PathBuf;

use super::Dimension;
use crate::config::tests::{Test, hours, project_only};

#[test]
fn partition_keys_parse() -> Test {
    let text = "storage.data_dir = \"/d\"\n\
        partition.by = [\"time\", \"ide\", \"project\"]\n\
        partition.time_bucket = \"12h\"\npartition.shards = 3\n";
    let c = project_only("partition", text)?;
    assert_eq!(c.storage.data_dir, Some(PathBuf::from("/d")));
    let by = vec![Dimension::Time, Dimension::Ide, Dimension::Project];
    let p = c.partition;
    assert_eq!((p.by, p.time_bucket, p.shards), (by, hours(12), 3));
    Ok(())
}
