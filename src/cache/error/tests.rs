use std::sync::Arc;
use std::thread;

use super::Error;
use crate::cache::tests::{Scratch, Test};
use crate::store;

#[test]
fn a_store_error_reads_as_the_store_says() {
    let e = Error::from(store::Error::NoDataDir);
    assert_eq!(e, Error::Store(store::Error::NoDataDir));
    assert_eq!(e.to_string(), store::Error::NoDataDir.to_string());
}

#[test]
fn a_poisoned_lock_says_what_to_do() {
    let text = Error::Poisoned.to_string();
    assert!(text.starts_with("chert: cache lock poisoned"), "{text}");
    assert!(text.ends_with("(restart chert mcp)"), "{text}");
}

#[test]
fn a_panic_holding_the_lock_is_an_error_not_a_panic() -> Test {
    let s = Scratch::new("poison")?;
    let cold = s.cold(&s.config()?)?;
    let inner = Arc::clone(&cold.handle.inner);
    let crashed = thread::spawn(move || {
        let _held = inner.lock();
        std::panic::resume_unwind(Box::new("held the lock"));
    })
    .join();
    assert!(crashed.is_err());
    assert_eq!(cold.handle.is_warm(), Err(Error::Poisoned));
    let q = store::Filter::default();
    assert_eq!(cold.handle.query(&q), Err(Error::Poisoned));
    Ok(())
}
