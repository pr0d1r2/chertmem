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
fn a_store_error_converts() {
    let e: Error = store::Error::NoDataDir.into();
    assert_eq!(e, Error::Store(store::Error::NoDataDir));
}
