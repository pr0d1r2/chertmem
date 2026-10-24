use std::path::Path;

use rusqlite::ffi;

use super::Error;

fn failure(code: i32) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(ffi::Error::new(code), None)
}

fn all() -> [Error; 9] {
    [
        Error::NoDataDir,
        Error::io(Path::new("x"), &std::io::Error::other("gone")),
        Error::Sqlite { reason: "r".into() },
        Error::Busy { reason: "r".into() },
        Error::WriteOnly,
        Error::ReadOnly { path: "p".into() },
        Error::BadRef { text: "t".into() },
        Error::Ambiguous {
            reference: "chert:R".into(),
            projects: vec!["a".into(), "b".into()],
        },
        Error::BadTime { ts: 1 },
    ]
}

/// The variants added in waves 4 and 5.
fn later() -> [Error; 2] {
    [
        Error::RefTaken {
            reference: "R".into(),
            id: "I".into(),
        },
        Error::NoId {
            ts: 1,
            reason: "r".into(),
        },
    ]
}

#[test]
fn every_message_reads_what_then_fix() {
    for e in all().into_iter().chain(later()) {
        let text = e.to_string();
        assert!(text.starts_with("chert: "), "{text}");
        assert!(text.ends_with(')') && text.contains(" ("), "{text}");
    }
}

#[test]
fn ambiguous_names_every_project() {
    let e = Error::Ambiguous {
        reference: "chert:01ABCDEF".into(),
        projects: vec!["one".into(), "two".into()],
    };
    assert!(e.to_string().contains("chert:01ABCDEF is in one, two"));
}

#[test]
fn busy_and_locked_map_to_busy() {
    for code in [ffi::SQLITE_BUSY, ffi::SQLITE_LOCKED] {
        let e = Error::from(failure(code));
        assert!(matches!(e, Error::Busy { .. }), "{e:?}");
    }
}

#[test]
fn other_sqlite_failures_stay_sqlite() {
    let e = Error::from(failure(ffi::SQLITE_CONSTRAINT));
    assert!(matches!(e, Error::Sqlite { .. }), "{e:?}");
}
