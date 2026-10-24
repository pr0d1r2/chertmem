use super::super::Intensity;
use super::{expansions, grammar_for};

type Res = Result<(), Box<dyn std::error::Error>>;

const LEVELS: [Intensity; 3] =
    [Intensity::Lite, Intensity::Full, Intensity::Ultra];

#[test]
fn off_has_no_grammar() {
    assert!(grammar_for(Intensity::Off).is_none());
}

#[test]
fn removals_are_sorted_longest_first() -> Res {
    for level in LEVELS {
        let grammar = grammar_for(level).ok_or("missing grammar")?;
        for list in &grammar.removals {
            let sorted = list
                .windows(2)
                .all(|w| matches!(w, [a, b] if a.len() >= b.len()));
            assert!(sorted, "{level:?}: {list:?}");
        }
    }
    Ok(())
}

/// Removal list sizes (pleasantries, hedges, fillers, articles), then
/// the number of abbreviations.
fn sizes(level: Intensity) -> Result<Vec<usize>, &'static str> {
    let grammar = grammar_for(level).ok_or("missing grammar")?;
    let mut out: Vec<usize> = grammar.removals.iter().map(Vec::len).collect();
    out.push(grammar.abbreviations.iter().map(|p| p.len()).sum());
    Ok(out)
}

#[test]
fn lexicon_sizes_match_cavemem() -> Res {
    assert_eq!(sizes(Intensity::Lite)?, [7, 0, 12, 0, 5]);
    assert_eq!(sizes(Intensity::Full)?, [8, 6, 20, 3, 36]);
    assert_eq!(sizes(Intensity::Ultra)?, [8, 11, 26, 3, 49]);
    Ok(())
}

#[test]
fn ultra_abbreviates_configuration_harder() {
    let first = |level| {
        let grammar = grammar_for(level)?;
        grammar.abbreviations.first()?.first().copied()
    };
    assert_eq!(first(Intensity::Lite), Some(("configuration", "config")));
    assert_eq!(first(Intensity::Full), Some(("configuration", "config")));
    assert_eq!(first(Intensity::Ultra), Some(("configuration", "cfg")));
}

#[test]
fn expansions_are_longest_first_and_paired() {
    let (keys, targets) = expansions();
    assert_eq!((keys.len(), targets.len()), (47, 47));
    let sorted = keys
        .windows(2)
        .all(|w| matches!(w, [a, b] if a.len() >= b.len()));
    assert!(sorted);
    let pair = keys.iter().zip(targets).find(|(k, _)| **k == "w/o");
    assert_eq!(pair, Some((&"w/o", &"without")));
}
