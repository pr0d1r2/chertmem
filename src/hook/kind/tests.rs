use super::{Ide, Kind};

#[test]
fn every_kind_parses_back_from_its_cli_name() {
    for kind in Kind::ALL {
        assert_eq!(Kind::parse(kind.as_str()), Some(kind));
    }
    let names: Vec<_> = Kind::ALL.iter().map(|k| k.as_str()).collect();
    let cli = [
        "session-start",
        "user-prompt-submit",
        "post-tool-use",
        "stop",
        "session-end",
    ];
    assert_eq!(names, cli);
    assert_eq!(Kind::parse("pre-tool-use"), None);
}

#[test]
fn claude_code_is_the_only_ide() {
    assert_eq!(Ide::parse("claude-code"), Some(Ide::ClaudeCode));
    assert_eq!(Ide::ClaudeCode.as_str(), "claude-code");
    assert_eq!(Ide::parse("cursor"), None);
}
