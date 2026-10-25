//! The machine a bench runs on (benches V3): CPU model and OS, nothing
//! that names a person or a host; and the date a file was written.

#[cfg(test)]
mod tests;

use std::path::Path;

use crate::record::Host;

/// This machine, `None` when its CPU model or OS cannot be read.
#[must_use]
pub fn this() -> Option<Host> {
    None
}

/// `cmd args`' trimmed stdout when it succeeds and is not empty.
fn output(cmd: &str, args: &[&str]) -> Option<String> {
    let _ = (cmd, args);
    None
}

/// The UTC date `file` was last written, `YYYY-MM-DD`.
#[must_use]
pub fn date_of(file: &Path) -> Option<String> {
    let _ = file;
    output("date", &[])
}
