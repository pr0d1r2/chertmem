//! Install error messages: `chert: <what> (<fix>)`.

use std::path::PathBuf;

use super::Error;
use crate::store;

const BY_HAND: &str = "(fix the file by hand, then run it again)";

fn path() -> PathBuf {
    PathBuf::from("s.json")
}

#[test]
fn an_io_error_names_the_path_and_reason() {
    let reason = "denied".to_owned();
    let got = Error::Io {
        path: path(),
        reason,
    }
    .to_string();
    let want = "chert: s.json: denied (check the path and its permissions)";
    assert_eq!(got, want);
}

#[test]
fn a_json_error_asks_for_a_fix_by_hand() {
    let reason = "eof".to_owned();
    let got = Error::Json {
        path: path(),
        reason,
    }
    .to_string();
    assert_eq!(got, format!("chert: s.json: eof {BY_HAND}"));
}

#[test]
fn a_shape_error_names_the_key() {
    let key = "hooks".to_owned();
    let got = Error::Shape { path: path(), key }.to_string();
    let want = "chert: s.json: `hooks` has the wrong JSON type";
    assert_eq!(got, format!("{want} {BY_HAND}"));
}

#[test]
fn a_store_error_reads_as_the_store_s_own() {
    let inner = store::Error::NoDataDir;
    let got = Error::from(inner.clone());
    assert_eq!(got.to_string(), inner.to_string());
    assert_eq!(got.what(), inner.to_string());
    assert_eq!(got.fix(), "run chert doctor");
}
