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
    named(std::fs::read_to_string("/proc/cpuinfo").ok().as_deref())
}

/// The CPU `/proc/cpuinfo` names, read or not; `uname -m` when it is
/// unreadable or names none.
#[cfg(any(test, not(target_os = "macos")))]
fn named(info: Option<&str>) -> Option<String> {
    info.and_then(model).or_else(|| output("uname", &["-m"]))
}

/// The CPU model a Linux `/proc/cpuinfo` names.
#[cfg(any(test, not(target_os = "macos")))]
fn model(info: &str) -> Option<String> {
    field(info, "model name").or_else(|| arm(info))
}

/// An ARM kernel prints no `model name`, only who designed each core and
/// which part it is (Neoverse N1: implementer 0x41, part 0xd0c). A mixed
/// host names every distinct core, sorted, so its class is its own.
#[cfg(any(test, not(target_os = "macos")))]
fn arm(info: &str) -> Option<String> {
    let parts = values(info, "CPU implementer").zip(values(info, "CPU part"));
    let cores: std::collections::BTreeSet<String> = parts
        .map(|(i, p)| format!("implementer {i} part {p}"))
        .collect();
    let cores: Vec<String> = cores.into_iter().collect();
    (!cores.is_empty()).then(|| format!("ARM {}", cores.join(" + ")))
}

/// The trimmed value of the first `key : value` line whose key is `key`.
#[cfg(any(test, not(target_os = "macos")))]
fn field(info: &str, key: &str) -> Option<String> {
    values(info, key).next().map(str::to_owned)
}

/// Every non-empty trimmed value of a `key : value` line keyed `key`.
#[cfg(any(test, not(target_os = "macos")))]
fn values<'a>(
    info: &'a str,
    key: &'a str,
) -> impl Iterator<Item = &'a str> + 'a {
    info.lines()
        .filter_map(move |l| l.split_once(':').filter(|(k, _)| k.trim() == key))
        .map(|(_, v)| v.trim())
        .filter(|v| !v.is_empty())
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
