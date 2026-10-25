//! `chert`, the chertmem binary: a shim over the library's `cli::main`
//! (src/cli G).

fn main() -> std::process::ExitCode {
    chertmem::cli::main()
}
