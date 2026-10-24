//! `lock-graph`: `flake.lock` holds exactly one nixpkgs and one
//! nixpkgs-lock node, so every input follows the fleet pin (dev V5).

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use tinyjson::JsonValue;

/// Why the lock graph fails, one line per broken rule; empty when it
/// holds. A lock that does not parse is reported as such.
#[must_use]
pub fn findings(lock: &str) -> Vec<String> {
    let Some(nodes) = nodes(lock) else {
        return vec!["flake.lock does not parse as JSON".to_owned()];
    };
    ["nixpkgs", "nixpkgs-lock"]
        .iter()
        .filter_map(|repo| once(&nodes, repo))
        .collect()
}

/// The lock's nodes, by name.
fn nodes(lock: &str) -> Option<HashMap<String, JsonValue>> {
    let root: JsonValue = lock.parse().ok()?;
    let map: &HashMap<String, JsonValue> = root.get()?;
    let nodes: &HashMap<String, JsonValue> = map.get("nodes")?.get()?;
    Some(nodes.clone())
}

/// A finding unless exactly one node's `original.repo` is `repo`.
fn once(nodes: &HashMap<String, JsonValue>, repo: &str) -> Option<String> {
    let mut names: Vec<&str> = nodes
        .iter()
        .filter(|(_, n)| original_repo(n) == Some(repo))
        .map(|(k, _)| k.as_str())
        .collect();
    names.sort_unstable();
    (names.len() != 1).then(|| {
        format!(
            "{} {repo} nodes ({}), want 1",
            names.len(),
            names.join(", ")
        )
    })
}

fn original_repo(node: &JsonValue) -> Option<&str> {
    let map: &HashMap<String, JsonValue> = node.get()?;
    let orig: &HashMap<String, JsonValue> = map.get("original")?.get()?;
    let repo: &String = orig.get("repo")?.get()?;
    Some(repo.as_str())
}
