use super::{Commit, exempt, findings};
use std::collections::BTreeSet;

const ID_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ID_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn commit(id: &str, subject: &str, added: &[&str]) -> Commit {
    Commit {
        id: id.to_owned(),
        subject: subject.to_owned(),
        added: added.iter().map(|s| (*s).to_owned()).collect(),
    }
}

#[test]
fn exempt_reads_full_ids_with_reasons() {
    let text = format!("# exempt\n\n{ID_A} stub crate, no behaviour (B1)\n");
    let want: BTreeSet<String> = [ID_A.to_owned()].into_iter().collect();
    assert_eq!(exempt(&text), Ok(want));
}

#[test]
fn exempt_refuses_a_short_id() {
    assert_eq!(
        exempt("abc123 short\n"),
        Err(
            "line 1: an exempt commit is named by its full 40-character id"
                .to_owned()
        )
    );
}

#[test]
fn exempt_refuses_an_entry_without_reason() {
    assert_eq!(
        exempt(&format!("{ID_A}\n")),
        Err(format!("line 1: {ID_A} has no reason"))
    );
}

#[test]
fn test_commit_first_holds() {
    let c = [
        commit(
            ID_A,
            "test(dev): RED x",
            &["dev/src/x.rs", "dev/src/x/tests.rs"],
        ),
        commit(ID_B, "feat(dev): GREEN x", &["dev/src/y.rs"]),
    ];
    assert_eq!(findings(&c, &BTreeSet::new()), Vec::<String>::new());
}

#[test]
fn planted_code_before_any_test_is_a_finding() {
    let c = [commit(ID_A, "feat(dev): x", &["dev/src/x.rs"])];
    let got = findings(&c, &BTreeSet::new());
    assert_eq!(got.len(), 1);
    assert!(
        got.iter()
            .all(|f| f.starts_with("aaaaaaa adds dev/src/x.rs"))
    );
}

#[test]
fn exempt_commit_and_tests_trees_hold() {
    let c = [
        commit(ID_A, "build(crate): stub", &["src/lib.rs"]),
        commit(
            ID_B,
            "chore: fixtures",
            &["tests/it.rs", "dev/tests/x.rs", "README.md"],
        ),
    ];
    let ex: BTreeSet<String> = [ID_A.to_owned()].into_iter().collect();
    assert_eq!(findings(&c, &ex), Vec::<String>::new());
}
