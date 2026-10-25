use super::stdout;
use std::path::Path;

type Res = Result<(), String>;

const SH: &str = "sh";

#[test]
fn stdout_of_a_command_that_holds() -> Res {
    let got = stdout(Path::new("."), SH, &["-c", "echo out; echo noise >&2"])?;
    assert_eq!(got, "out\n");
    Ok(())
}

/// The planted violation of rekall#47: a failing command is an error
/// with its exit code and stderr, never an empty stdout.
#[test]
fn a_failing_command_keeps_its_status_and_stderr() {
    let got = stdout(Path::new("."), SH, &["-c", "echo boom >&2; exit 101"]);
    let want = "`sh -c echo boom >&2; exit 101` failed (exit 101): boom";
    assert_eq!(got, Err(want.to_owned()));
}

#[test]
fn a_command_that_cannot_start_or_prints_bytes_is_an_error() {
    let missing = stdout(Path::new("."), "chertmem-dev-no-such-tool", &[]);
    assert!(missing.is_err_and(|e| e.contains("did not start")));
    let bytes = stdout(Path::new("."), SH, &["-c", "printf '\\377'"]);
    assert!(bytes.is_err_and(|e| e.contains("non-UTF-8")));
    let killed = stdout(Path::new("."), SH, &["-c", "kill -9 $$"]);
    assert!(killed.is_err_and(|e| e.contains("a signal")));
}
