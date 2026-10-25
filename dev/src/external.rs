//! External commands that fail loudly (dev T11, rekall#47): a command
//! that does not start, exits non-zero or prints non-UTF-8 is an error
//! carrying the command line and its stderr, never empty output.

#[cfg(test)]
mod tests;

use std::path::Path;
use std::process::{Command, Output};

/// `program args` run in `dir`: its stdout when it exits 0.
///
/// # Errors
/// The command did not start, exited non-zero (its stderr in the error)
/// or printed non-UTF-8.
pub fn stdout(
    dir: &Path,
    program: &str,
    args: &[&str],
) -> Result<String, String> {
    let line = std::iter::once(program)
        .chain(args.iter().copied())
        .collect::<Vec<_>>()
        .join(" ");
    let out = Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("`{line}` did not start: {e}"))?;
    if !out.status.success() {
        return Err(failed(&line, &out));
    }
    String::from_utf8(out.stdout)
        .map_err(|_| format!("`{line}` printed non-UTF-8"))
}

/// Why `line` failed: its exit code, or the signal, and its stderr.
fn failed(line: &str, out: &Output) -> String {
    let code = out
        .status
        .code()
        .map_or("a signal".to_owned(), |c| format!("exit {c}"));
    let err = String::from_utf8_lossy(&out.stderr);
    format!("`{line}` failed ({code}): {}", err.trim())
}
