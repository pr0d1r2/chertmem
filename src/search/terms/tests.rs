use super::{Stats, Terms};
use crate::search::tests::{BASE, DAY, Scratch, Test, event};
use crate::store::{Filter, Store};

fn words(t: &[&str]) -> Vec<String> {
    t.iter().map(|&w| w.to_owned()).collect()
}

/// 3 rows in 3 weekly partitions: 9 tokens.
fn three(store: &mut Store) -> Test {
    store.append(&event(BASE, "s", "cargo build failed"))?;
    store.append(&event(BASE + 8 * DAY, "s", "cargo test cargo"))?;
    store.append(&event(BASE + 16 * DAY, "s", "git push now"))?;
    Ok(())
}

#[test]
fn counts_every_row_across_partitions() -> Test {
    let s = Scratch::new("terms-all")?;
    let mut store = s.open(&s.config()?)?;
    three(&mut store)?;
    assert_eq!(store.partitions(&Filter::default())?.len(), 3);
    let mut terms = Terms::open(&s.data())?;
    terms.refresh(&mut store)?;
    let got = terms.stats(&words(&["cargo", "git", "absent"]))?;
    let want = Stats {
        docs: 3,
        tokens: 9,
        df: vec![2, 1, 0],
    };
    assert_eq!(got, want);
    Ok(())
}

#[test]
fn a_refresh_counts_only_new_rows() -> Test {
    let s = Scratch::new("terms-tail")?;
    let mut store = s.open(&s.config()?)?;
    three(&mut store)?;
    let mut terms = Terms::open(&s.data())?;
    terms.refresh(&mut store)?;
    terms.refresh(&mut store)?;
    store.append(&event(BASE + DAY, "s", "cargo again"))?;
    store.append(&event(BASE + 40 * DAY, "s", "new bucket cargo"))?;
    terms.refresh(&mut store)?;
    let got = terms.stats(&words(&["cargo"]))?;
    assert_eq!((got.docs, got.tokens, got.df), (5, 14, vec![4]));
    Ok(())
}

#[test]
fn a_dropped_partition_is_recounted_without_its_rows() -> Test {
    let s = Scratch::new("terms-drop")?;
    let mut store = s.open(&s.config()?)?;
    three(&mut store)?;
    let mut terms = Terms::open(&s.data())?;
    terms.refresh(&mut store)?;
    let first = store.partitions(&Filter::default())?;
    let path = first.first().map(|p| p.path.clone()).ok_or("none")?;
    store.drop(&path)?;
    terms.refresh(&mut store)?;
    let got = terms.stats(&words(&["cargo", "failed"]))?;
    assert_eq!((got.docs, got.tokens, got.df), (2, 6, vec![1, 0]));
    Ok(())
}

#[test]
fn two_handles_on_1_file_count_each_row_once() -> Test {
    let s = Scratch::new("terms-shared")?;
    let cfg = s.config()?;
    let (mut a, mut b) = (s.open(&cfg)?, s.open(&cfg)?);
    three(&mut a)?;
    let (mut ta, mut tb) = (Terms::open(&s.data())?, Terms::open(&s.data())?);
    ta.refresh(&mut a)?;
    tb.refresh(&mut b)?;
    drop(ta);
    let again = Terms::open(&s.data())?;
    assert_eq!(again.stats(&words(&["cargo"]))?.df, [2]);
    assert_eq!(tb.stats(&[])?.docs, 3);
    Ok(())
}

#[test]
fn an_unusable_dir_is_an_error() -> Test {
    let s = Scratch::new("terms-bad")?;
    std::fs::write(s.data(), "a file, not a dir")?;
    assert!(Terms::open(&s.data()).is_err());
    Ok(())
}

#[test]
fn a_counted_partition_the_catalog_lost_forces_a_recount() -> Test {
    let s = Scratch::new("terms-lost")?;
    let mut store = s.open(&s.config()?)?;
    three(&mut store)?;
    let mut terms = Terms::open(&s.data())?;
    terms.refresh(&mut store)?;
    let sql = "INSERT INTO parts (path, seq) VALUES ('gone.db', 9);
      UPDATE terms SET df = 99 WHERE term = 'cargo'";
    terms.conn.execute_batch(sql)?;
    terms.refresh(&mut store)?;
    assert_eq!(terms.stats(&words(&["cargo"]))?.df, [2]);
    Ok(())
}
