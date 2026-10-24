//! `chertmem-dev`: entry point. Dispatch lives in the library.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    ExitCode::from(chertmem_dev::cli::run(&args, std::path::Path::new(".")))
}
