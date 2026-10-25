use super::{ALPHABET, held_out, nanos_per_kib, random_refs, report, texts};
use crate::hook::inject::estimate::{self, piece};
use crate::hook::inject::{HEADER, tokens_at_most};
use crate::hook::tests::Test;

/// The worst count over exact o200k on the corpus, in thousandths (R8):
/// the vocabulary is a rank prefix, not fitted to the corpus.
const WORST: usize = 1_040;

/// The worst on the held-out sources (R8, the owner's target).
const HELD_OUT: usize = 1_030;

/// The line every injected session starts with.
const LINE: &str = "- chert:";

#[test]
fn the_count_never_undercounts_o200k_on_the_corpus() -> Test {
    let texts = texts()?;
    assert!(texts.len() > 1_000, "{}", texts.len());
    for text in texts {
        let (counted, exact) = (estimate::tokens(text), tokens_at_most(text));
        assert!(counted >= exact, "{counted} < {exact}: {text}");
    }
    assert!(report(texts)? <= WORST);
    Ok(())
}

#[test]
fn the_count_is_within_3_percent_on_the_held_out_sources() -> Test {
    let held = held_out()?;
    assert!(held.len() > 50, "{}", held.len());
    for text in &held {
        assert!(estimate::tokens(text) >= tokens_at_most(text), "{text}");
    }
    assert!(report(&held)? <= HELD_OUT);
    Ok(())
}

/// The count's speed (R8): `--release` for the recorded figure.
#[test]
fn the_count_takes_its_time_per_kib() -> Test {
    let nanos = nanos_per_kib(texts()?, 5)?;
    println!("{nanos} ns per KiB");
    assert!(nanos > 0);
    Ok(())
}

#[test]
fn the_header_and_line_start_count_as_o200k_does() {
    for text in [HEADER, LINE, "", "hello world"] {
        assert_eq!(estimate::tokens(text), tokens_at_most(text), "{text}");
    }
}

/// Every ref the store may issue, 8 chars up to a whole ULID, counts as
/// o200k counts it (src/hook T14 step 1), after a session line's start
/// and before a gist or the line's end.
#[test]
fn every_ref_counts_as_o200k_does() {
    for len in 8..=26 {
        for reference in random_refs(0x9E37_79B9_7F4A_7C15, len, 2_000) {
            for text in [
                format!("{LINE}{reference}"),
                format!("{LINE}{reference} fix the parser\n"),
            ] {
                let exact = tokens_at_most(&text);
                assert_eq!(estimate::tokens(&text), exact, "{text}");
            }
        }
    }
}

/// Every run of up to 3 ref capitals, after the `:` of a line's start
/// as a ref's first piece, or after a digit (src/hook T14 step 1).
#[test]
fn every_short_run_of_ref_capitals_counts_as_o200k_does() {
    let mut runs = vec![String::new()];
    for _ in 1..=3 {
        runs = longer(&runs);
        let texts = runs
            .iter()
            .flat_map(|r| [format!(":{r}0"), format!("0{r}")]);
        for text in texts {
            let exact = tokens_at_most(&text);
            assert_eq!(estimate::tokens(&text), exact, "{text}");
        }
    }
}

/// Every run of `runs` with 1 more ref capital.
fn longer(runs: &[String]) -> Vec<String> {
    let capitals = ALPHABET.iter().filter(|b| b.is_ascii_uppercase());
    let capitals: Vec<char> = capitals.map(|&b| char::from(b)).collect();
    let grown = runs
        .iter()
        .flat_map(|run| capitals.iter().map(move |c| format!("{run}{c}")));
    grown.collect()
}

/// Every run of 1 to 3 digits, the other pieces of a ref.
#[test]
fn every_run_of_digits_counts_as_o200k_does() {
    for n in 0..1_000 {
        for text in [format!("{n}"), format!("{n:02}"), format!("{n:03}")] {
            assert_eq!(estimate::tokens(&text), tokens_at_most(&text));
        }
    }
}

#[test]
fn o200k_counts_each_piece_apart() -> Test {
    for text in texts()? {
        let chars: Vec<char> = text.chars().collect();
        let apart: usize = piece::pieces(&chars)
            .map(|(_, of)| tokens_at_most(&of.iter().collect::<String>()))
            .sum();
        assert_eq!(apart, tokens_at_most(text), "{text}");
    }
    Ok(())
}
