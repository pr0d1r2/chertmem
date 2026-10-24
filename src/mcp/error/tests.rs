use super::Error;
use crate::{search, store};

#[test]
fn store_and_search_errors_read_as_their_own() {
    let e = Error::from(store::Error::NoDataDir);
    assert_eq!(e.to_string(), store::Error::NoDataDir.to_string());
    let inner = search::Error::Thread;
    assert_eq!(Error::from(inner.clone()).to_string(), inner.to_string());
}

#[test]
fn a_stdio_failure_says_what_to_do() {
    let io = std::io::Error::other("pipe closed");
    let text = Error::from(io).to_string();
    assert!(text.starts_with("chert: mcp stdio failed: pipe"), "{text}");
    assert!(text.ends_with("(restart chert mcp)"), "{text}");
}
