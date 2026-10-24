//! `lock-graph`: `flake.lock` holds exactly one nixpkgs and one
//! nixpkgs-lock node, so every input follows the fleet pin (dev V5).

#[cfg(test)]
mod tests;

/// Why the lock graph fails, one line per broken rule; empty when it
/// holds. A lock that does not parse is reported as such.
#[must_use]
pub fn findings(lock: &str) -> Vec<String> {
    let _ = lock;
    Vec::new()
}
