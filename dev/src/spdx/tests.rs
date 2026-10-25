use super::choose;

type Res = Result<(), String>;

#[test]
fn mit_is_taken_whenever_it_is_offered() -> Res {
    for expr in [
        "MIT",
        "MIT OR Apache-2.0",
        "Apache-2.0 OR MIT",
        "MIT/Apache-2.0",
        "Unlicense OR MIT",
        "MIT OR Apache-2.0 OR LGPL-2.1-or-later",
        "(Apache-2.0 OR MIT)",
    ] {
        assert_eq!(choose(expr)?, "MIT", "{expr}");
    }
    Ok(())
}

#[test]
fn an_and_term_stays_beside_the_choice() -> Res {
    let got = choose("(MIT OR Apache-2.0) AND Unicode-3.0")?;
    assert_eq!(got, "MIT AND Unicode-3.0");
    let got = choose("Unicode-3.0 AND (Apache-2.0 OR MIT)")?;
    assert_eq!(got, "Unicode-3.0 AND MIT");
    Ok(())
}

#[test]
fn without_mit_the_most_permissive_option_wins() -> Res {
    assert_eq!(choose("BSD-3-Clause")?, "BSD-3-Clause");
    assert_eq!(choose("Apache-2.0 OR BSD-3-Clause")?, "BSD-3-Clause");
    assert_eq!(choose("GPL-2.0-or-later OR Zlib")?, "Zlib");
    assert_eq!(choose("Apache-2.0 OR Nonsense-1.0")?, "Apache-2.0");
    let exc = "Apache-2.0 WITH LLVM-exception OR MPL-2.0";
    assert_eq!(choose(exc)?, "Apache-2.0 WITH LLVM-exception");
    Ok(())
}

#[test]
fn a_broken_expression_is_an_error() {
    for expr in ["", "MIT OR", "(MIT", "MIT)", "MIT WITH", "AND MIT"] {
        let got = choose(expr);
        assert!(got.is_err(), "{expr}: {got:?}");
    }
    assert!(choose("MIT)").is_err_and(|e| e.contains("`)`")));
}
