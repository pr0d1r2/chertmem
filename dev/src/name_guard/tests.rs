use super::{Finding, names, scan};

fn file(path: &str, text: &str) -> (String, String) {
    (path.to_owned(), text.to_owned())
}

#[test]
fn names_skip_blanks_and_comments_and_lowercase() {
    let got = names("# private names\n\nAcme Corp\n  secretproj  \n");
    assert_eq!(got, vec!["acme corp".to_owned(), "secretproj".to_owned()]);
}

#[test]
fn planted_name_is_found_by_file_and_line() {
    let deny = names("secretproj\n");
    let files = [file("README.md", "intro\nsee SecretProj docs\n")];
    let want = vec![Finding {
        path: "README.md".to_owned(),
        line: 2,
    }];
    assert_eq!(scan(&deny, &files), want);
}

#[test]
fn clean_twin_holds() {
    let deny = names("secretproj\n");
    let files = [file("README.md", "intro\nsee public docs\n")];
    assert_eq!(scan(&deny, &files), Vec::new());
}

#[test]
fn empty_denylist_holds() {
    let files = [file("README.md", "anything at all\n")];
    assert_eq!(scan(&names(""), &files), Vec::new());
}
