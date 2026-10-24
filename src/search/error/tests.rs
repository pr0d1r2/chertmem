use std::path::Path;

use super::Error;
use crate::{cache, store};

fn own() -> [Error; 6] {
    [
        Error::io(Path::new("terms.db"), &std::io::Error::other("gone")),
        Error::from(rusqlite::Error::InvalidQuery),
        Error::Thread,
        Error::Embed { reason: "r".into() },
        Error::Unsupported { provider: "openai" },
        Error::Dropped {
            reference: "chert:01ABCDEF".into(),
        },
    ]
}

#[test]
fn every_own_message_reads_what_then_fix() {
    for e in own() {
        let text = e.to_string();
        assert!(text.starts_with("chert: "), "{text}");
        assert!(text.ends_with(')') && text.contains(" ("), "{text}");
    }
}

#[test]
fn store_and_cache_errors_read_as_they_say() {
    let e = Error::from(store::Error::NoDataDir);
    assert_eq!(e.to_string(), store::Error::NoDataDir.to_string());
    let c = Error::from(cache::Error::Poisoned);
    assert_eq!(c.to_string(), cache::Error::Poisoned.to_string());
}

#[test]
fn an_io_failure_names_the_file() {
    let e = Error::io(Path::new("terms.db"), &std::io::Error::other("x"));
    assert!(e.to_string().contains("terms.db: x"), "{e}");
}

#[test]
fn wrapped_errors_add_no_text_of_their_own() {
    for e in [
        Error::Store(store::Error::WriteOnly),
        Error::Cache(cache::Error::Poisoned),
    ] {
        assert_eq!((e.what(), e.fix()), (String::new(), ""));
    }
}
