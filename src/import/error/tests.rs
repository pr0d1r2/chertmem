use super::Error;
use crate::store;

/// Each error with a part its message must hold.
fn cases() -> [(Error, &'static str); 3] {
    let io = std::io::Error::other("disk full");
    let reason = "no such table".to_owned();
    [
        (Error::Store(store::Error::WriteOnly), "write-only"),
        (Error::Cavemem { reason }, "unreadable: no such table (give"),
        (Error::io(&io), "export failed: disk full (check"),
    ]
}

#[test]
fn every_message_names_what_and_fix() {
    for (e, part) in cases() {
        let text = e.to_string();
        assert!(text.starts_with("chert: "), "{text}");
        assert!(text.contains(part), "{text}");
    }
}

#[test]
fn an_sqlite_error_is_a_cavemem_error() {
    let e: Error = rusqlite::Error::QueryReturnedNoRows.into();
    assert!(matches!(e, Error::Cavemem { .. }));
}

#[test]
fn a_line_error_names_its_line_and_cause() {
    let reason = "expected value".to_owned();
    let e = Error::Line {
        line: 7,
        cause: Box::new(Error::Jsonl { reason }),
    };
    let text = e.to_string();
    let want = "chert: import line 7: not an export line: expected value";
    assert!(text.starts_with(want), "{text}");
    assert!(
        text.ends_with("(give a file `chert export` wrote)"),
        "{text}"
    );
}

#[test]
fn a_store_error_converts() {
    let e: Error = store::Error::NoDataDir.into();
    assert_eq!(e, Error::Store(store::Error::NoDataDir));
}
