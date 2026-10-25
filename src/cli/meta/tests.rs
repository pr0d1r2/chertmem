//! `chert completions` and `chert man`: generated from the 1 clap
//! definition, exit 0; an unknown shell is exit 2 (V1, V2).

use super::super::tests::{Sandbox, Test};

#[test]
fn completions_for_each_shell_name_every_verb() -> Test {
    let s = Sandbox::new("completions")?;
    for shell in ["bash", "zsh", "fish", "elvish", "powershell"] {
        let ran = s.run(&["completions", shell]);
        assert_eq!(ran.code, 0, "{shell}: {ran:?}");
        for verb in ["timeline", "partitions", "completions"] {
            assert!(ran.out.contains(verb), "{shell} lacks {verb}");
        }
    }
    Ok(())
}

#[test]
fn an_unknown_shell_is_exit_2() -> Test {
    let s = Sandbox::new("completions-bad")?;
    let ran = s.run(&["completions", "tcsh"]);
    assert_eq!(ran.code, 2, "{ran:?}");
    Ok(())
}

#[test]
fn man_is_a_roff_page_naming_every_verb() -> Test {
    let s = Sandbox::new("man")?;
    let ran = s.run(&["man"]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert!(ran.out.starts_with(".ie") || ran.out.contains(".TH chert"));
    for verb in ["search", "doctor", "export"] {
        assert!(ran.out.contains(verb), "man lacks {verb}");
    }
    Ok(())
}
