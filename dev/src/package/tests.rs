use super::findings;

const LIST: &str = "Cargo.lock\nCargo.toml\nLICENSE\nREADME.md\n\
                    THIRD-PARTY-NOTICES.md\nsrc/lib.rs\nsrc/main.rs\n";

const README: &str = "# x\n\n[MIT](LICENSE), [notices](THIRD-PARTY-NOTICES.md#rust-crates),\n\
                      [spec](https://github.com/o/r/blob/main/SPEC.md), [top](#x).\n\n\
                      [ref]: src/lib.rs\n";

#[test]
fn a_package_of_sources_and_licenses_holds() {
    assert_eq!(findings(LIST, README), Vec::<String>::new());
}

/// dev V6: the planted violations are a SPEC.md and a hook in the
/// tarball.
#[test]
fn a_spec_or_a_hook_in_the_package_is_a_finding() {
    let list =
        format!("{LIST}src/store/SPEC.md\nSPEC.md\n.githooks/pre-commit\n");
    let got = findings(&list, README);
    let want = [
        "src/store/SPEC.md ships; keep it out of `include`",
        "SPEC.md ships; keep it out of `include`",
        ".githooks/pre-commit ships; keep it out of `include`",
    ];
    assert_eq!(got, want);
    let near = format!("{LIST}src/SPEC.md.bak\ndocs/NOTSPEC.md\n");
    assert!(findings(&near, README).is_empty());
}

#[test]
fn a_missing_license_file_is_a_finding() {
    let list = LIST.replace("THIRD-PARTY-NOTICES.md\n", "");
    let got = findings(&list, README);
    assert!(
        got.contains(
            &"THIRD-PARTY-NOTICES.md does not ship; add it".to_owned()
        ),
        "{got:?}"
    );
}

#[test]
fn a_readme_link_to_a_file_left_out_is_a_finding() {
    let readme =
        "[a](docs/A.md) [b](CONTRIBUTING.md#x) [c](docs/A.md)\n[d]: SPEC.md\n";
    let got = findings(LIST, readme);
    let want: Vec<String> = ["CONTRIBUTING.md", "SPEC.md", "docs/A.md"]
        .iter()
        .map(|l| format!("README.md links to {l}, which does not ship; use an absolute URL"))
        .collect();
    assert_eq!(got, want);
}
