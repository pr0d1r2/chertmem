use super::core_worktree;

#[test]
fn planted_core_worktree_is_found() {
    let cfg = "[core]\n\tbare = false\n\tworktree = /tmp/elsewhere\n";
    assert_eq!(core_worktree(cfg), Some("/tmp/elsewhere".to_owned()));
}

#[test]
fn clean_twin_holds() {
    let cfg = "[core]\n\tbare = false\n\thooksPath = .githooks\n";
    assert_eq!(core_worktree(cfg), None);
}

#[test]
fn worktree_key_in_another_section_is_not_core() {
    let cfg = "[core]\n\tbare = false\n[extensions]\n\tworktree = x\n";
    assert_eq!(core_worktree(cfg), None);
}

#[test]
fn section_and_key_names_are_case_insensitive() {
    let cfg = "[Core]\n\tWorkTree = /w\n";
    assert_eq!(core_worktree(cfg), Some("/w".to_owned()));
}
