use std::path::Path;

use super::Error;
use crate::store;

#[test]
fn every_message_names_what_and_fix() {
    let path = Path::new("data/mine/findings.db");
    let cases = [
        (Error::Store(store::Error::WriteOnly), "write-only"),
        (
            Error::ledger(path, "disk full"),
            "findings.db unusable: disk full (remove it;",
        ),
    ];
    for (e, part) in cases {
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
