//! The machine a bench runs on (benches V3): CPU model and OS, nothing
//! that names a person or a host; and the date a file was written.

#[cfg(test)]
mod tests;

use std::path::Path;
use std::process::Command;

use crate::record::Host;

/// This machine, `None` when its CPU model or OS cannot be read.
#[must_use]
pub fn this() -> Option<Host> {
    Some(Host {
        machine: machine()?,
        os: os()?,
    })
}

/// `cmd args`' trimmed stdout when it succeeds and is not empty.
fn output(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    let text = text.trim();
    (out.status.success() && !text.is_empty()).then(|| text.to_owned())
}

#[cfg(target_os = "macos")]
fn machine() -> Option<String> {
    output("sysctl", &["-n", "machdep.cpu.brand_string"])
}

#[cfg(target_os = "macos")]
fn os() -> Option<String> {
    output("sw_vers", &["-productVersion"]).map(|v| format!("macOS {v}"))
}

#[cfg(not(target_os = "macos"))]
fn machine() -> Option<String> {
    let info = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    let line = info.lines().find(|l| l.starts_with("model name"))?;
    Some(line.split_once(':')?.1.trim().to_owned())
}

#[cfg(not(target_os = "macos"))]
fn os() -> Option<String> {
    output("uname", &["-sr"])
}

/// The UTC date `file` was last written, `YYYY-MM-DD`: `date -r`, which
/// GNU and BSD `date` both read as a file's time.
#[must_use]
pub fn date_of(file: &Path) -> Option<String> {
    let file = file.to_str()?;
    output("date", &["-u", "-r", file, "+%Y-%m-%d"])
}
