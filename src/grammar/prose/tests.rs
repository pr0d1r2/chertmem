use super::super::Intensity;
use super::super::lexicon::grammar_for;
use super::{
    blank_lines, blanks, collapse, compress_prose, edge_spaces, newline,
    rewrite, space_marks, tighten,
};

type Res = Result<(), Box<dyn std::error::Error>>;

#[test]
fn blank_runs_become_one_space() {
    assert_eq!(rewrite("a \t b\t\tc", blanks), "a b c");
}

#[test]
fn one_space_either_side_of_a_newline_goes() {
    assert_eq!(rewrite("a \n b\n\nc  \nd", newline), "a\nb\n\nc \nd");
}

#[test]
fn spaces_before_a_mark_go_but_tabs_stay() {
    assert_eq!(rewrite("a ,b\t;c  ?d e", space_marks), "a,b\t;c?d e");
    assert_eq!(tighten("a ,b\t;c \t?d e"), "a,b;c?d e");
}

#[test]
fn three_or_more_newlines_become_two() {
    assert_eq!(
        rewrite("a\n\nb\n\n\nc\n\n\n\n\nd", blank_lines),
        "a\n\nb\n\nc\n\nd"
    );
}

#[test]
fn spaces_at_either_end_of_a_line_go() {
    let text = "  a  b \nc \rd\u{2028} e\u{2029}  ";
    assert_eq!(rewrite(text, edge_spaces), "a  b\nc\rd\u{2028}e\u{2029}");
}

#[test]
fn collapse_runs_every_step_in_order() {
    assert_eq!(collapse(" a \t ,\n\n\n\n  b  "), "a,\n\nb");
}

#[test]
fn white_space_only_prose_is_kept() -> Res {
    let grammar = grammar_for(Intensity::Full).ok_or("no full grammar")?;
    assert_eq!(compress_prose(" \n\t ", grammar), " \n\t ");
    Ok(())
}

#[test]
fn edges_pad_to_a_newline_a_space_or_nothing() -> Res {
    let grammar = grammar_for(Intensity::Full).ok_or("no full grammar")?;
    assert_eq!(compress_prose("\t \n the end  ", grammar), "\nend ");
    assert_eq!(compress_prose("the end", grammar), "end");
    assert_eq!(compress_prose("the\n", grammar), "\n");
    Ok(())
}

#[test]
fn phrases_then_abbreviations_keep_case() -> Res {
    let grammar = grammar_for(Intensity::Ultra).ok_or("no ultra grammar")?;
    let text = "I think that the Configuration and CONFIGURATION";
    assert_eq!(compress_prose(text, grammar), "that Cfg & CFG");
    Ok(())
}
