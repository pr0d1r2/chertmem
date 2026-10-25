use super::{random_refs, report, texts};
use crate::hook::inject::estimate::{self, HEADER_TOKENS, LINE, LINE_TOKENS};
use crate::hook::inject::{HEADER, tokens_at_most};
use crate::hook::tests::Test;

/// The worst estimate over exact o200k on the corpus, in thousandths
/// (R7): a looser estimate fails here, as an undercount fails at 1000.
const WORST: usize = 1_250;

#[test]
fn the_estimate_never_undercounts_o200k_on_the_corpus() -> Test {
    let texts = texts()?;
    assert!(texts.len() > 1_000, "{}", texts.len());
    for text in texts {
        let (estimate, exact) = (estimate::tokens(text), tokens_at_most(text));
        assert!(estimate >= exact, "{estimate} < {exact}: {text}");
    }
    let worst = report(texts)?;
    assert!(worst <= WORST, "{worst}");
    Ok(())
}

#[test]
fn the_header_and_line_start_count_as_o200k_does() {
    assert_eq!(tokens_at_most(HEADER), HEADER_TOKENS);
    assert_eq!(tokens_at_most(LINE), LINE_TOKENS);
}

#[test]
fn no_ref_takes_more_than_its_shape_counts() {
    for len in 8..=14 {
        let mut most = 0;
        for reference in random_refs(0x9E37_79B9_7F4A_7C15, len, 5_000) {
            let exact = tokens_at_most(&format!("{LINE}{reference}"));
            let counted = estimate::reference_tokens(&reference);
            assert!(exact <= counted, "{reference}: {exact} > {counted}");
            most = most.max(exact);
        }
        assert!(most <= LINE_TOKENS.saturating_add(len), "{len}: {most}");
    }
}

#[test]
fn every_run_of_ref_capitals_takes_at_most_1_token_each() {
    let capitals: Vec<char> = estimate::ALPHABET
        .chars()
        .filter(char::is_ascii_uppercase)
        .collect();
    let mut runs = vec![String::new()];
    for len in 1..=3_usize {
        let grown = runs
            .iter()
            .flat_map(|run| capitals.iter().map(move |c| format!("{run}{c}")));
        runs = grown.collect();
        for run in &runs {
            let exact = tokens_at_most(&format!("0{run}0"));
            assert!(exact <= len.saturating_add(2), "{run}: {exact}");
        }
    }
}

#[test]
fn o200k_counts_each_piece_apart() -> Test {
    for text in texts()? {
        let chars: Vec<char> = text.chars().collect();
        let apart: usize = estimate::piece::pieces(&chars)
            .map(|(_, of)| tokens_at_most(&of.iter().collect::<String>()))
            .sum();
        assert_eq!(apart, tokens_at_most(text), "{text}");
    }
    Ok(())
}
