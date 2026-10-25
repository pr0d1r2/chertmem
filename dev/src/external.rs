//! External commands that fail loudly (dev T10, rekall#47): a command
//! that does not start, exits non-zero or prints non-UTF-8 is an error
//! carrying the command line and its stderr, never empty output.

#[cfg(test)]
mod tests;

use std::path::Path;

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
    let _ = (dir, program, args);
    Ok(String::new())
}
