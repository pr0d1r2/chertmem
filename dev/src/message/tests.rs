use super::findings;

const TRAILERS: &str = "Paradigm-Stage: wave-1\n\
                        Co-Authored-By: A <a@example.com>\n";

/// A message with `subject`, a one-line body and the trailer paragraph.
fn message(subject: &str) -> String {
    format!("{subject}\n\nWhy it changed.\n\n{TRAILERS}")
}

/// Whether there is exactly one finding and it contains `part`.
fn only(text: &str, part: &str) -> bool {
    let found = findings(text);
    found.len() == 1 && found.iter().all(|f| f.contains(part))
}

#[test]
fn clean_message_holds() {
    assert!(findings(&message("feat(dev): add a guard (T8)")).is_empty());
}

#[test]
fn scopeless_and_breaking_subjects_hold() {
    assert!(findings(&message("fix: a claim")).is_empty());
    assert!(findings(&message("refactor(src/store)!: a claim")).is_empty());
    assert!(findings(&message("perf!: a claim")).is_empty());
}

#[test]
fn every_type_holds() {
    let types = [
        "feat", "fix", "docs", "test", "chore", "refactor", "perf", "build",
        "ci",
    ];
    for t in types {
        assert!(findings(&message(&format!("{t}: x"))).is_empty(), "{t}");
    }
}

#[test]
fn unknown_type_is_a_finding() {
    assert!(only(&message("feature(dev): x"), "type(scope): claim"));
    assert!(only(&message("Feat: x"), "type(scope): claim"));
}

#[test]
fn missing_claim_is_a_finding() {
    assert!(only(&message("feat(dev):"), "type(scope): claim"));
    assert!(only(&message("feat(dev): "), "type(scope): claim"));
    assert!(only(&message("feat(dev):x"), "type(scope): claim"));
}

#[test]
fn malformed_scope_is_a_finding() {
    assert!(only(&message("feat(): x"), "type(scope): claim"));
    assert!(only(&message("feat(Dev): x"), "type(scope): claim"));
    assert!(only(&message("feat(dev: x"), "type(scope): claim"));
}

#[test]
fn subject_of_72_characters_holds() {
    let subject = format!("feat: {}", "x".repeat(66));
    assert_eq!(subject.chars().count(), 72);
    assert!(findings(&message(&subject)).is_empty());
}

#[test]
fn subject_of_73_characters_is_a_finding() {
    let subject = format!("feat: {}", "x".repeat(67));
    assert!(only(&message(&subject), "73 characters"));
}

#[test]
fn length_counts_characters_not_bytes() {
    let subject = format!("feat: {}", "\u{e9}".repeat(66));
    assert!(subject.len() > 72);
    assert!(findings(&message(&subject)).is_empty());
}

#[test]
fn git_subjects_pass() {
    for s in ["Merge branch 'x'", "Revert \"x\"", "fixup! x", "squash! x"] {
        assert!(findings(&format!("{s}\n")).is_empty(), "{s}");
    }
}

#[test]
fn empty_body_is_a_finding() {
    assert!(only(&format!("feat: x\n\n{TRAILERS}"), "body"));
}

#[test]
fn blank_body_lines_are_no_body() {
    assert!(only(&format!("feat: x\n\n  \n\n{TRAILERS}"), "body"));
}

#[test]
fn missing_trailer_is_a_finding() {
    let text = "feat: x\n\nWhy.\n\nCo-Authored-By: A <a@example.com>\n";
    assert!(only(text, "Paradigm-Stage"));
}

#[test]
fn trailer_outside_the_final_paragraph_is_a_finding() {
    let text = "feat: x\n\nWhy.\n\nParadigm-Stage: wave-1\n\nMore prose.\n";
    assert!(only(text, "Paradigm-Stage"));
}

#[test]
fn trailer_beside_prose_is_not_parsed() {
    let text = "feat: x\n\nWhy.\n\nsome prose\nParadigm-Stage: wave-1\n";
    assert!(only(text, "Paradigm-Stage"));
}

#[test]
fn trailer_glued_to_the_subject_is_no_trailer() {
    let found = findings("feat: x\nParadigm-Stage: wave-1\n");
    assert!(found.iter().any(|f| f.contains("Paradigm-Stage")));
}

#[test]
fn trailing_blank_lines_are_ignored() {
    let text = format!("{}\n\n", message("feat: x"));
    assert!(findings(&text).is_empty());
}

const STAGES: [&str; 12] = [
    "intent",
    "gap-round-1",
    "spec-run",
    "ck-check-round-12",
    "dev-shell",
    "guardrails",
    "wave-3",
    "reconcile-2",
    "gate",
    "release",
    "backprop",
    "propagation",
];

#[test]
fn every_stage_holds() {
    for s in STAGES {
        let text = format!("feat: x\n\nWhy.\n\nParadigm-Stage: {s}\n");
        assert!(findings(&text).is_empty(), "{s}");
    }
}

#[test]
fn unknown_stages_are_findings() {
    for s in ["wave", "wave-0", "wave-01", "wave-x", "gap-round-", "ship"] {
        let text = format!("feat: x\n\nWhy.\n\nParadigm-Stage: {s}\n");
        assert!(only(&text, "not a paradigm stage"), "{s}");
    }
}

#[test]
fn comments_and_scissors_are_dropped() {
    let text = format!(
        "# Please enter the commit message\n{}# a comment\n\
         # ------------------------ >8 ------------------------\n\
         diff --git a b\n",
        message("feat: x")
    );
    assert!(findings(&text).is_empty());
}

#[test]
fn each_broken_rule_is_one_finding() {
    let subject = format!("oops {}", "x".repeat(80));
    assert_eq!(findings(&format!("{subject}\n")).len(), 4);
}

#[test]
fn empty_message_breaks_every_rule() {
    assert_eq!(findings("").len(), 3);
}
