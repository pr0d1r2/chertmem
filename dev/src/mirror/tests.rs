use super::{bodied_fns, findings, wired};
use std::collections::BTreeMap;

const WIRED: &str = "#[cfg(test)]\nmod tests;\n";
const LOGIC: &str = "pub fn f() -> u8 {\n    1\n}\n";

fn tree(files: &[(&str, &str)]) -> BTreeMap<String, String> {
    files
        .iter()
        .map(|(p, t)| ((*p).to_owned(), (*t).to_owned()))
        .collect()
}

fn roots() -> Vec<String> {
    vec![String::new(), "dev/".to_owned()]
}

#[test]
fn bodied_fns_skip_signatures_and_comments() {
    let text = "// fn commented() {}\ntrait T { fn sig(&self); }\n\
                pub fn real(x: u8) -> u8 { x }\nfn g<T: Into<u8>>(t: T) {}\n";
    assert_eq!(bodied_fns(text), vec!["real".to_owned(), "g".to_owned()]);
}

#[test]
fn wired_needs_the_cfg_test_mod_declaration() {
    assert!(wired("#[cfg(test)]\nmod tests;\n"));
    assert!(wired("#[cfg(test)] mod   tests ;"));
    assert!(!wired("mod tests;\n"));
    assert!(!wired("// #[cfg(test)] mod tests;\n"));
}

#[test]
fn wired_module_with_mirror_holds() {
    let rs = tree(&[
        ("src/a.rs", &format!("{WIRED}{LOGIC}")),
        ("src/a/tests.rs", "#[test]\nfn t() {}\n"),
    ]);
    assert_eq!(findings(&rs, &roots()), Vec::<String>::new());
}

#[test]
fn planted_module_without_mirror_is_a_finding() {
    let rs = tree(&[("src/a.rs", LOGIC)]);
    let got = findings(&rs, &roots());
    assert_eq!(got.len(), 1);
    assert!(
        got.iter()
            .all(|f| f.contains("src/a.rs has logic and no unit tests"))
    );
}

#[test]
fn planted_unwired_mirror_is_a_finding() {
    let rs = tree(&[("src/a.rs", LOGIC), ("src/a/tests.rs", "")]);
    let got = findings(&rs, &roots());
    assert!(
        got.iter()
            .any(|f| f.contains("src/a/tests.rs is never compiled"))
    );
}

#[test]
fn planted_orphan_mirror_is_a_finding() {
    let rs = tree(&[("src/gone/tests.rs", "")]);
    let got = findings(&rs, &roots());
    assert!(
        got.iter()
            .any(|f| f.contains("src/gone/tests.rs tests nothing"))
    );
}

#[test]
fn lib_rs_mirror_is_tests_rs_beside_it() {
    let rs = tree(&[
        ("src/lib.rs", &format!("{WIRED}{LOGIC}")),
        ("src/tests.rs", ""),
    ]);
    assert_eq!(findings(&rs, &roots()), Vec::<String>::new());
}

#[test]
fn pure_wiring_and_shim_main_hold() {
    let rs = tree(&[
        ("src/lib.rs", "pub mod a;\npub use a::f;\n"),
        ("src/main.rs", "fn main() {\n    chertmem::run();\n}\n"),
    ]);
    assert_eq!(findings(&rs, &roots()), Vec::<String>::new());
}

#[test]
fn planted_logic_in_main_is_a_finding() {
    let rs = tree(&[("src/main.rs", "fn main() {}\nfn helper() {}\n")]);
    let got = findings(&rs, &roots());
    assert!(got.iter().any(|f| f.contains("src/main.rs is not a shim")));
}

#[test]
fn build_vendor_and_integration_tests_are_exempt() {
    let rs = tree(&[
        ("build.rs", LOGIC),
        ("vendor/x.rs", LOGIC),
        ("tests/it.rs", LOGIC),
        ("dev/tests/it.rs", LOGIC),
    ]);
    assert_eq!(findings(&rs, &roots()), Vec::<String>::new());
}
