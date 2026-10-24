use super::ON_PROMPT_DISCLAIMER;
use crate::config::Error as ConfigError;
use crate::config::tests::{Scratch, Test, project_error, project_only};

#[test]
fn inject_defaults_to_300_tokens_off_prompt() -> Test {
    let inject = Scratch::new("injectdefault")?.load()?.config.inject;
    assert_eq!((inject.max_tokens, inject.on_prompt), (300, false));
    Ok(())
}

#[test]
fn inject_keys_parse() -> Test {
    let text = "[inject]\nmax_tokens = 500\non_prompt = true\n";
    let inject = project_only("injectkeys", text)?.inject;
    assert_eq!((inject.max_tokens, inject.on_prompt), (500, true));
    Ok(())
}

#[test]
fn bad_inject_values_are_errors() -> Test {
    let cases = ["inject.max_tokens = -1", "inject.on_prompt = \"yes\""];
    for (i, line) in cases.iter().enumerate() {
        let e = project_error(&format!("badinject{i}"), &format!("{line}\n"))?;
        assert!(matches!(e, ConfigError::Invalid { .. }), "{line}: {e}");
    }
    Ok(())
}

#[test]
fn turning_on_prompt_on_gives_the_disclaimer_once() -> Test {
    let mut inject = Scratch::new("disclaimer")?.load()?.config.inject;
    assert_eq!(inject.set_on_prompt(true), Some(ON_PROMPT_DISCLAIMER));
    assert!(inject.on_prompt);
    assert_eq!(inject.set_on_prompt(true), None);
    assert_eq!(inject.set_on_prompt(false), None);
    assert!(!inject.on_prompt);
    Ok(())
}

#[test]
fn disclaimer_names_the_wait_and_the_cost() {
    let text = ON_PROMPT_DISCLAIMER;
    assert!(text.starts_with("chert: inject.on_prompt = true"), "{text}");
    assert!(text.contains("every prompt"), "{text}");
    assert!(text.contains("store read"), "{text}");
    assert!(text.contains("spool replay"), "{text}");
    assert!(text.contains("docs/BENCHMARKS.md"), "{text}");
}
