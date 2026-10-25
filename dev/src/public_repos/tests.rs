use super::{Allowlist, Unlisted, refs, unlisted};

type Res = Result<(), String>;

const LIST: &str = r#"
self = "pr0d1r2/chertmem"

[[repo]]
name = "pr0d1r2/rekall"
public = "2026-10-25"
"#;

fn file(path: &str, text: &str) -> (String, String) {
    (path.to_owned(), text.to_owned())
}

#[test]
fn listed_and_self_are_allowed_ignoring_case() -> Res {
    let list = Allowlist::parse(LIST)?;
    assert!(list.allows("pr0d1r2/rekall"));
    assert!(list.allows("PR0D1R2/Rekall"));
    assert!(list.allows("pr0d1r2/chertmem"));
    assert!(!list.allows("pr0d1r2/secret"));
    Ok(())
}

#[test]
fn a_row_without_a_date_or_name_is_refused() {
    let undated = "[[repo]]\nname = \"a/b\"\n";
    assert!(Allowlist::parse(undated).is_err());
    let bad = "[[repo]]\nname = \"a/b\"\npublic = \"yesterday\"\n";
    assert!(Allowlist::parse(bad).is_err());
    assert!(Allowlist::parse("[[repo]]\npublic = \"2026-10-25\"\n").is_err());
    assert!(Allowlist::parse("other = \"x\"\n").is_err());
    assert!(Allowlist::parse("not toml [").is_err());
}

#[test]
fn refs_read_links_flake_refs_and_owner_paths() {
    let text = "see https://github.com/jdx/hk.git.\n\
                url = \"github:pr0d1r2/sherd/v0.5.4\"\n\
                pr0d1r2/rekall#46 and src/store\n\
                https://github.com/pr0d1r2/pklith/blob/main/x\n";
    let want = vec![
        (1, "jdx/hk".to_owned()),
        (2, "pr0d1r2/sherd".to_owned()),
        (3, "pr0d1r2/rekall".to_owned()),
        (4, "pr0d1r2/pklith".to_owned()),
    ];
    assert_eq!(refs(text), want);
}

#[test]
fn planted_unlisted_repo_is_found_and_listed_twin_holds() -> Res {
    let list = Allowlist::parse(LIST)?;
    let planted = [file("a.md", "ok pr0d1r2/rekall\nsee pr0d1r2/secret\n")];
    let want = vec![Unlisted {
        path: "a.md".to_owned(),
        line: 2,
        repo: "pr0d1r2/secret".to_owned(),
    }];
    assert_eq!(unlisted(&list, &planted), want);
    let twin = [file("a.md", "ok pr0d1r2/rekall\nsee pr0d1r2/chertmem\n")];
    assert_eq!(unlisted(&list, &twin), Vec::new());
    Ok(())
}
