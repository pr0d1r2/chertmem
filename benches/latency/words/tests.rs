// Only `#[test]` fns here, imports inside them: `cargo bench` builds
// this module with `--cfg test` and strips every test (lib.rs).

#[test]
fn the_head_comes_first_then_the_tail_is_spelled() {
    use super::word;
    assert_eq!(word(0), "the");
    assert_eq!(word(8), "cargo");
    let tail = word(64);
    assert!(tail.len() >= 4 && tail.chars().all(|c| c.is_ascii_lowercase()));
    assert_ne!(word(600).len(), word(100).len());
}

#[test]
fn every_rank_spells_a_distinct_word() {
    use super::{HEAD, VOCABULARY, word};
    let all: std::collections::BTreeSet<String> =
        (0..VOCABULARY).map(word).collect();
    let n = usize::try_from(VOCABULARY).unwrap_or_default();
    assert_eq!(all.len(), n);
    assert!(HEAD.iter().all(|w| all.contains(*w)));
}

#[test]
fn common_words_outnumber_rare_ones() {
    use super::{any, word};
    let rng = &mut crate::rng::Rng::new(11);
    let drawn: Vec<String> = (0..20_000).map(|_| any(rng)).collect();
    let count = |w: &str| drawn.iter().filter(|d| *d == w).count();
    assert!(count("the") > count(&word(2000)).saturating_add(10));
}

#[test]
fn sentences_have_the_words_asked_for() {
    use super::{prose, sentence};
    let rng = &mut crate::rng::Rng::new(2);
    assert_eq!(sentence(rng, 7).split(' ').count(), 7);
    let n = prose(rng, 3, 4).split(' ').count();
    assert!((3..7).contains(&n), "{n}");
}

#[test]
fn paths_look_like_a_repository() {
    let rng = &mut crate::rng::Rng::new(4);
    let paths: Vec<String> = (0..200).map(|_| super::path(rng)).collect();
    let starts = ["src/", "tests/", "docs/"];
    assert!(
        paths
            .iter()
            .all(|p| starts.iter().any(|s| p.starts_with(s)))
    );
    let ext =
        |p: &String| std::path::Path::new(p).extension().map(|e| e == "md");
    assert!(paths.iter().any(|p| ext(p) == Some(true)));
    assert!(paths.iter().filter(|p| p.starts_with("src/")).count() > 100);
}
