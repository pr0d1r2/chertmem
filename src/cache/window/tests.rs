use super::Window;
use crate::cache::tests::{DAY, HOUR, NOW, START, Scratch, Test, event, from};
use crate::store::{Filter, Mode};

#[test]
fn begin_lists_the_partitions_of_the_window_only() -> Test {
    let s = Scratch::new("window-list")?;
    let mut store = s.open(Mode::Read)?;
    store.append(&event(START - 30 * DAY, "far before"))?;
    store.append(&event(NOW - HOUR, "inside"))?;
    let window = Window::begin(&store, START)?;
    let inside = store.partitions(&from(&store, START))?;
    let paths: Vec<String> = inside.into_iter().map(|p| p.path).collect();
    assert_eq!(window.paths(), paths);
    assert_eq!(paths.len(), 1);
    Ok(())
}

#[test]
fn sync_loads_a_partition_and_answer_keeps_ts_id_order() -> Test {
    let s = Scratch::new("window-sync")?;
    let mut store = s.open(Mode::Read)?;
    store.append(&event(NOW - HOUR, "b"))?;
    store.append(&event(NOW - 3 * HOUR, "a"))?;
    store.append(&event(START - DAY, "before"))?;
    let mut window = Window::begin(&store, START)?;
    for path in window.paths() {
        window.sync(&mut store, &path)?;
    }
    let q = from(&store, START);
    let rows = window.answer(&q).ok_or("declined")?;
    assert_eq!(rows, store.observations(&q)?);
    let bodies: Vec<&str> = rows.iter().map(|o| o.body.as_str()).collect();
    assert_eq!(bodies, ["a", "b"]);
    Ok(())
}

#[test]
fn answer_declines_what_the_window_may_not_hold() -> Test {
    let s = Scratch::new("window-decline")?;
    let store = s.open(Mode::Read)?;
    let window = Window::begin(&store, START)?;
    let mut other = from(&store, START);
    other.project = None;
    for q in [Filter::default(), from(&store, START - 1), other] {
        assert_eq!(window.answer(&q), None, "{q:?}");
    }
    assert_eq!(window.answer(&from(&store, START)), Some(Vec::new()));
    Ok(())
}
