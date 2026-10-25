//! `package-list` (dev T11): what `cargo package -p chertmem --list`
//! puts in the crates.io tarball. No `SPEC.md` and nothing under
//! `.githooks/` ships, every file building and licensing need does, and
//! every relative link in `README.md` points at a file that ships, so
//! none breaks on crates.io.

#[cfg(test)]
mod tests;

/// The files the tarball must hold.
pub const REQUIRED: [&str; 5] = [
    "Cargo.lock",
    "Cargo.toml",
    "LICENSE",
    "README.md",
    "THIRD-PARTY-NOTICES.md",
];

/// Why the package `list` (1 path per line) fails, given the README's
/// text; empty when it holds.
#[must_use]
pub fn findings(list: &str, readme: &str) -> Vec<String> {
    let files: Vec<&str> = list.lines().map(str::trim).collect();
    let mut found: Vec<String> = files
        .iter()
        .filter(|f| forbidden(f))
        .map(|f| format!("{f} ships; keep it out of `include`"))
        .collect();
    let missing = REQUIRED.iter().filter(|r| !files.contains(r));
    found.extend(missing.map(|r| format!("{r} does not ship; add it")));
    let broken = links(readme).into_iter().filter(|l| !files.contains(l));
    found.extend(broken.map(|l| {
        format!(
            "README.md links to {l}, which does not ship; use an absolute URL"
        )
    }));
    found
}

/// A spec or a hook: development files, not the crate's.
fn forbidden(path: &str) -> bool {
    path.starts_with(".githooks/")
        || path.rsplit('/').next().is_some_and(|f| f == "SPEC.md")
}

/// Relative link targets in `md`, fragment dropped, each once.
fn links(md: &str) -> Vec<&str> {
    let inline = md.split("](").skip(1).filter_map(|s| s.split(')').next());
    let refs = md.lines().filter_map(|l| {
        let rest = l.trim_start().strip_prefix('[')?;
        rest.split_once("]: ").map(|(_, t)| t.trim())
    });
    let mut out: Vec<&str> = inline
        .chain(refs)
        .map(|t| t.split('#').next().unwrap_or(t))
        .filter(|t| !t.is_empty() && !t.contains(':'))
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}
