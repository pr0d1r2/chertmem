//! The memory figure the `cache.max_bytes` check reads (src/cache C,
//! R2): `MemAvailable` on Linux; on macOS the total, as it has no cheap
//! "available" figure.

#[cfg(test)]
mod tests;

use std::process::Command;

/// Bytes of memory available; `None` on another system or when the
/// figure cannot be read.
pub(super) fn available() -> Option<u64> {
    if cfg!(target_os = "linux") {
        let text = std::fs::read_to_string("/proc/meminfo").ok()?;
        meminfo(&text)
    } else if cfg!(target_os = "macos") {
        let out = Command::new("/usr/sbin/sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .ok()?;
        memsize(&String::from_utf8_lossy(&out.stdout))
    } else {
        None
    }
}

/// `MemAvailable` of `/proc/meminfo`, in bytes.
fn meminfo(_text: &str) -> Option<u64> {
    // RED stub: no figure read yet.
    None
}

/// The output of `sysctl -n hw.memsize`: bytes.
fn memsize(_text: &str) -> Option<u64> {
    // RED stub: no figure read yet.
    None
}
