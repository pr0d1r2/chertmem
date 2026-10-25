// Only `#[test]` fns here, imports inside them: `cargo bench` builds
// this module with `--cfg test` and strips every test (lib.rs).

#[test]
fn exactly_size_events_by_time() {
    use super::{Row, Rows};
    let rows = Rows::new(crate::fixture::Spec::new(1, 1234, 4));
    let ts: Vec<u64> = rows
        .filter_map(|r| match r {
            Row::Event(e) => Some(e.ts),
            Row::Begin(_) | Row::Summary(_) => None,
        })
        .collect();
    assert_eq!(ts.len(), 1234);
    assert!(ts.windows(2).all(|w| w.first() <= w.last()));
}

#[test]
fn a_session_opens_with_its_start() {
    use super::{Row, Rows};
    let rows: Vec<Row> =
        Rows::new(crate::fixture::Spec::new(2, 400, 1)).collect();
    let starts = rows.windows(2).filter_map(|w| match (w.first(), w.last()) {
        (Some(Row::Begin(b)), Some(Row::Event(e))) => Some((b, e)),
        _ => None,
    });
    let starts: Vec<_> = starts.collect();
    assert!(starts.len() > 3);
    for (b, e) in starts {
        assert_eq!(
            (b.ts, &b.id, e.kind.as_str()),
            (e.ts, &e.session, "session-start")
        );
    }
}

#[test]
fn a_session_closes_with_its_summary() {
    use super::{Row, Rows};
    let rows: Vec<Row> =
        Rows::new(crate::fixture::Spec::new(2, 400, 1)).collect();
    let ends = rows.windows(2).filter_map(|w| match (w.first(), w.last()) {
        (Some(Row::Event(e)), Some(Row::Summary(s))) => Some((e, s)),
        _ => None,
    });
    let ends: Vec<_> = ends.collect();
    assert!(ends.len() > 3);
    for (e, s) in ends {
        assert_eq!(
            (e.ts, &e.session, e.kind.as_str()),
            (s.ts, &s.session, "session-end")
        );
    }
}

#[test]
fn every_kind_occurs_and_only_tool_calls_carry_an_identity() {
    use super::{Row, Rows};
    let events: Vec<chertmem::store::Event> =
        Rows::new(crate::fixture::Spec::new(3, 2000, 1))
            .filter_map(|r| if let Row::Event(e) = r { Some(e) } else { None })
            .collect();
    for kind in chertmem::hook::Kind::ALL {
        assert!(events.iter().any(|e| e.kind == kind.as_str()), "{kind:?}");
    }
    let calls = ["post-tool-use", "post-tool-use-failure"];
    let call = |e: &chertmem::store::Event| calls.contains(&e.kind.as_str());
    assert!(events.iter().all(|e| e.identity.is_some() == call(e)));
    let ids: std::collections::BTreeSet<_> =
        events.iter().filter_map(|e| e.identity.as_ref()).collect();
    assert_eq!(ids.len(), events.iter().filter(|e| call(e)).count());
}

#[test]
fn tool_bodies_are_name_input_outcome_as_the_hook_stores_them() {
    use super::{Row, Rows};
    let tools: Vec<String> = Rows::new(crate::fixture::Spec::new(4, 500, 1))
        .filter_map(|r| match r {
            Row::Event(e) if e.kind == "post-tool-use" => Some(e.body),
            _ => None,
        })
        .collect();
    let names = ["Bash", "Read", "Edit", "Grep"];
    let json = |l: &str| serde_json::from_str::<serde_json::Value>(l).is_ok();
    assert!(tools.iter().all(|b| b.lines().count() == 3));
    assert!(tools.iter().all(|b| b.lines().nth(1).is_some_and(json)));
    assert!(names.iter().all(|n| tools.iter().any(|b| b.starts_with(n))));
}
