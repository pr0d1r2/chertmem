use super::{HEADER, Row, ms, os_name, parse, render};

type Res = Result<(), Box<dyn std::error::Error>>;

fn row(suite: &str, fixture: &str, median_ms: f64) -> Row {
    Row {
        suite: suite.to_owned(),
        fixture: fixture.to_owned(),
        target: Some("get_observations".to_owned()),
        median_ms,
        p95_ms: median_ms,
        tolerance: 0.5,
        machine: "Test CPU".to_owned(),
        os: "TestOS 1.2".to_owned(),
        date: "2026-10-25".to_owned(),
        recount: format!("cargo bench -- '^{suite}/{fixture}$'"),
    }
}

#[test]
fn a_rendered_baseline_parses_back_to_its_rows() -> Res {
    let mut untargeted = row("partition_scan", "1M-p52", 241.0);
    untargeted.target = None;
    let rows = vec![row("get_observations", "1k-p52", 0.0812), untargeted];
    let text = render(&rows);
    assert!(text.starts_with(HEADER));
    assert_eq!(parse(&text)?, rows);
    assert_eq!(render(&parse(&text)?), text, "the same bytes again");
    Ok(())
}

#[test]
fn a_row_is_written_key_by_key() {
    let text = render(&[row("get_observations", "1k-p52", 26.1)]);
    assert!(text.contains("\n[[row]]\nsuite = \"get_observations\"\n"));
    assert!(text.contains("\nmedian_ms = 26.10\np95_ms = 26.10\n"));
    assert!(text.contains("\ntolerance = 0.5\n"));
    assert!(text.contains("\nrecount = \"cargo bench -- '^get_"));
}

#[test]
fn numbers_keep_4_or_5_significant_digits() {
    assert_eq!(ms(241.04), "241.0");
    assert_eq!(ms(26.1), "26.10");
    assert_eq!(ms(1.23456), "1.235");
    assert_eq!(ms(0.08123), "0.08123");
    assert_eq!(ms(0.004_56), "0.00456");
}

#[test]
fn quotes_and_backslashes_are_escaped() -> Res {
    let mut r = row("s", "1k-p1", 1.0);
    r.recount = r#"say "hi" \ bye"#.to_owned();
    assert_eq!(parse(&render(&[r.clone()]))?, vec![r]);
    Ok(())
}

#[test]
fn a_bad_row_is_named_by_its_place() {
    let text = format!("{}[[row]]\nsuite = 1\n", render(&[row("s", "f", 1.0)]));
    let err = parse(&text).err().unwrap_or_default();
    assert!(err.starts_with("row 2: `suite` is not text"), "{err}");
    assert!(
        parse("[[row]]\nsuite = ").is_err_and(|e| e.starts_with("not TOML"))
    );
    assert!(parse("row = 1").is_err());
    assert!(parse("row = [1]").is_err_and(|e| e == "row 1: not a table"));
}

#[test]
fn negative_numbers_and_integers_are_told_apart() -> Res {
    let text = render(&[row("s", "f", 1.0)]);
    let negative = text.replace("tolerance = 0.5", "tolerance = -0.5");
    let err = parse(&negative).err().unwrap_or_default();
    assert!(
        err.contains("`tolerance` is not a number of at least 0"),
        "{err}"
    );
    let whole = text.replace("tolerance = 0.5", "tolerance = 1");
    assert!(
        (parse(&whole)?.first().map_or(0.0, |r| r.tolerance) - 1.0).abs()
            < 1e-9
    );
    Ok(())
}

#[test]
fn no_rows_is_an_empty_baseline() -> Res {
    assert!(parse(HEADER)?.is_empty());
    assert_eq!(render(&[]), HEADER);
    Ok(())
}

#[test]
fn the_class_leaves_the_os_version_out() {
    let r = row("s", "f", 1.0);
    assert_eq!(r.class(), ("Test CPU", "TestOS"));
    assert_eq!(os_name(""), "");
}
