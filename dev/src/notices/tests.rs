use super::{Entry, Meta, clean, metadata, render, shipped, splice, wanted};
use std::collections::BTreeSet;
use std::path::PathBuf;

type Res = Result<(), String>;

fn pair(name: &str, version: &str) -> (String, String) {
    (name.to_owned(), version.to_owned())
}

#[test]
fn shipped_reads_each_package_once() {
    let tree = "chertmem v0.0.0 (/repo)\nclap v4.6.7\nclap v4.6.7 (*)\n\
                serde_derive v1.0.229 (proc-macro)\n\nwarning\n";
    let want = BTreeSet::from([
        pair("chertmem", "0.0.0"),
        pair("clap", "4.6.7"),
        pair("serde_derive", "1.0.229"),
    ]);
    assert_eq!(shipped(tree), want);
}

const META: &str = r#"{"packages":[
 {"name":"clap","version":"4.6.7","license":"MIT OR Apache-2.0",
  "repository":"https://github.com/clap-rs/clap",
  "manifest_path":"/reg/clap-4.6.7/Cargo.toml",
  "source":"registry+https://github.com/rust-lang/crates.io-index"},
 {"name":"chertmem","version":"0.0.0","license":null,"repository":null,
  "manifest_path":"/repo/Cargo.toml","source":null}
]}"#;

#[test]
fn metadata_keeps_license_repository_dir_and_source() -> Res {
    let got = metadata(META)?;
    let clap = got.get(&pair("clap", "4.6.7")).ok_or("no clap")?;
    let want = Meta {
        license: Some("MIT OR Apache-2.0".to_owned()),
        repository: Some("https://github.com/clap-rs/clap".to_owned()),
        dir: PathBuf::from("/reg/clap-4.6.7"),
        registry: true,
    };
    assert_eq!(clap, &want);
    let own = got.get(&pair("chertmem", "0.0.0")).ok_or("no chertmem")?;
    assert_eq!((own.license.as_ref(), own.registry), (None, false));
    Ok(())
}

#[test]
fn metadata_of_another_shape_is_an_error() {
    let nameless = r#"{"packages":[{"version":"1","manifest_path":"/a"}]}"#;
    for (bad, why) in [
        ("[", ""),
        ("[]", "not an object"),
        ("{}", "no packages"),
        (r#"{"packages":[1]}"#, "a package is not"),
        (nameless, "has no name"),
    ] {
        let got = metadata(bad);
        assert!(
            got.as_ref().is_err_and(|e| e.contains(why)),
            "{bad}: {got:?}"
        );
    }
}

fn files(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

/// (a crate's files, the license chosen, the files that go on the page).
type Case = (
    &'static [&'static str],
    &'static str,
    &'static [&'static str],
);

const CASES: [Case; 7] = [
    (
        &["LICENSE-APACHE", "LICENSE-MIT", "src"],
        "MIT",
        &["LICENSE-MIT"],
    ),
    (
        &["LICENSE-UNICODE", "LICENSE-MIT", "LICENSE-APACHE"],
        "MIT AND Unicode-3.0",
        &["LICENSE-MIT", "LICENSE-UNICODE"],
    ),
    (
        &["LICENSE.BSD-3-Clause", "LICENSE", "build.rs"],
        "BSD-3-Clause",
        &["LICENSE", "LICENSE.BSD-3-Clause"],
    ),
    (
        &["COPYING", "LICENSE-MIT", "NOTICE.txt", "UNLICENSE"],
        "MIT",
        &["COPYING", "LICENSE-MIT", "NOTICE.txt"],
    ),
    (
        &["LICENSE-MIT", "LICENSE-APACHE"],
        "Zlib",
        &["LICENSE-APACHE", "LICENSE-MIT"],
    ),
    (&["licence.md", "LICENSE-APACHE"], "MIT", &["licence.md"]),
    (&["src", "Cargo.toml", "README.md"], "MIT", &[]),
];

#[test]
fn wanted_takes_the_chosen_licenses_files_and_generic_ones() {
    for (list, chosen, want) in CASES {
        assert_eq!(wanted(&files(list), chosen), files(want), "{list:?}");
    }
}

#[test]
fn clean_normalises_endings_tabs_and_edges() {
    let got = clean("\n\r\nMIT License  \r\n\n\tCopyright x\t\n\n");
    assert_eq!(got, "MIT License\n\n    Copyright x");
}

fn entry(name: &str, bpe_only: bool, texts: &[&str]) -> Entry {
    Entry {
        name: name.to_owned(),
        version: "1.0.0".to_owned(),
        license: "MIT".to_owned(),
        repository: (name != "b").then(|| format!("https://x.test/{name}")),
        bpe_only,
        texts: texts.iter().map(|t| (*t).to_owned()).collect(),
    }
}

fn block() -> String {
    render(&[
        entry("a", false, &["MIT a"]),
        entry("b", false, &["MIT a", "```code```"]),
        entry("c", true, &[]),
    ])
}

#[test]
fn render_lists_every_crate_with_its_texts_once() {
    let got = block();
    assert_eq!(block(), got, "deterministic");
    assert!(got.starts_with("<!-- BEGIN crates -->\n\n## Rust crates\n"));
    assert!(got.ends_with("```\n\n<!-- END crates -->\n"));
    assert_eq!(got.matches("MIT a").count(), 1);
    for line in [
        "That is 2 crates in the default build, and 1 more with\nfeature",
        "| a | 1.0.0 | MIT | default | <https://x.test/a> | [1](#text-1) |",
        "| b | 1.0.0 | MIT | default | - | [1](#text-1), [2](#text-2) |",
        "| c | 1.0.0 | MIT | `bpe` | <https://x.test/c> | none shipped |",
        "### Text 1\n\nShipped by `a 1.0.0`, `b 1.0.0`.\n\n```text\nMIT a\n```",
        "### Text 2\n\nShipped by `b 1.0.0`.\n\n````text\n```code```\n````",
    ] {
        assert!(got.contains(line), "{line}\n---\n{got}");
    }
}

#[test]
fn splice_replaces_only_the_block() -> Res {
    let doc = "# T\n\nhand\n\n<!-- BEGIN crates -->\nold\n<!-- END crates -->\ntail\n";
    let got = splice(doc, "<!-- BEGIN crates -->\nnew\n<!-- END crates -->\n")?;
    let want = "# T\n\nhand\n\n<!-- BEGIN crates -->\nnew\n<!-- END crates -->\ntail\n";
    assert_eq!(got, want);
    assert!(splice("# T\n", "x").is_err());
    assert!(splice("<!-- BEGIN crates -->\n", "x").is_err());
    Ok(())
}
