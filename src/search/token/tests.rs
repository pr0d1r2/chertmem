use super::{terms, tokens};
use crate::search::tests::{BASE, Test, event, fresh};
use crate::store::{Filter, Store};

#[test]
fn tokens_split_on_anything_not_alphanumeric_and_lowercase() {
    let got = tokens("Fix the cargo.lock -- Cargo  build_42!");
    let want = ["fix", "the", "cargo", "lock", "cargo", "build", "42"];
    assert_eq!(got, want);
}

#[test]
fn terms_are_distinct_and_sorted() {
    assert_eq!(terms("b a B c a"), ["a", "b", "c"]);
    assert!(terms(" -- ").is_empty());
}

#[test]
fn non_ascii_letters_are_tokens_too() {
    let upper = "\u{17b}\u{f3}\u{141}w pije";
    assert_eq!(tokens(upper), ["\u{17c}\u{f3}\u{142}w", "pije"]);
}

/// The only partition's path.
fn only(store: &mut Store) -> Result<String, Box<dyn std::error::Error>> {
    let parts = store.partitions(&Filter::default())?;
    Ok(parts.first().map(|p| p.path.clone()).ok_or("none")?)
}

/// Whether the index of `path` matches `term` in the row of `body`.
fn indexed(
    store: &mut Store,
    path: &str,
    term: &str,
    body: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let found = store.matches(path, &[term.to_owned()], &Filter::default())?;
    Ok(found.iter().any(|t| t.observation.body == body))
}

#[test]
fn the_index_matches_every_token_of_an_ascii_body() -> Test {
    let (_s, _cfg, mut store) = fresh("token-ascii")?;
    let bodies: Vec<String> = (1_u8..128)
        .map(|c| format!("Ab{}cD ef", char::from(c)))
        .collect();
    for body in &bodies {
        store.append(&event(BASE, "s", body))?;
    }
    let path = only(&mut store)?;
    for body in &bodies {
        for term in tokens(body) {
            assert!(indexed(&mut store, &path, &term, body)?, "{body:?}");
        }
    }
    Ok(())
}

#[test]
fn a_combining_mark_splits_a_search_token_not_an_index_token() -> Test {
    let (_s, _cfg, mut store) = fresh("token-mark")?;
    let body = "e\u{301}x";
    store.append(&event(BASE, "s", body))?;
    let path = only(&mut store)?;
    assert_eq!(tokens(body), ["e", "x"]);
    assert!(!indexed(&mut store, &path, "e", body)?);
    assert!(indexed(&mut store, &path, "ex", body)?);
    Ok(())
}
