use super::findings;

fn node(repo: &str) -> String {
    format!(
        r#"{{"locked":{{"owner":"o","repo":"{repo}","type":"github"}},"original":{{"owner":"o","repo":"{repo}","type":"github"}}}}"#
    )
}

fn lock(nodes: &[(&str, String)]) -> String {
    let body: Vec<String> =
        nodes.iter().map(|(k, v)| format!(r#""{k}":{v}"#)).collect();
    format!(
        r#"{{"nodes":{{"root":{{"inputs":{{}}}},{}}},"root":"root","version":7}}"#,
        body.join(",")
    )
}

#[test]
fn one_nixpkgs_and_one_pin_holds() {
    let l = lock(&[
        ("nixpkgs", node("nixpkgs")),
        ("nixpkgs-lock", node("nixpkgs-lock")),
        ("sherd", node("sherd")),
    ]);
    assert_eq!(findings(&l), Vec::<String>::new());
}

#[test]
fn planted_second_nixpkgs_is_a_finding() {
    let l = lock(&[
        ("nixpkgs", node("nixpkgs")),
        ("nixpkgs_2", node("nixpkgs")),
        ("nixpkgs-lock", node("nixpkgs-lock")),
    ]);
    assert_eq!(
        findings(&l),
        vec!["2 nixpkgs nodes (nixpkgs, nixpkgs_2), want 1".to_owned()]
    );
}

#[test]
fn planted_second_pin_is_a_finding() {
    let l = lock(&[
        ("nixpkgs", node("nixpkgs")),
        ("nixpkgs-lock", node("nixpkgs-lock")),
        ("nixpkgs-lock_2", node("nixpkgs-lock")),
    ]);
    assert_eq!(
        findings(&l),
        vec![
            "2 nixpkgs-lock nodes (nixpkgs-lock, nixpkgs-lock_2), want 1"
                .to_owned()
        ]
    );
}

#[test]
fn unparsable_lock_is_a_finding() {
    assert_eq!(
        findings("{not json"),
        vec!["flake.lock does not parse as JSON".to_owned()]
    );
}
