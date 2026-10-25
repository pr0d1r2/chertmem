//! The benches' own tests (benches V1, V2), run by `cargo nextest run`:
//! the modules `main.rs` benches with, built under libtest. `cargo
//! bench` builds `main.rs` with `--cfg test` but no harness, so a test
//! module there holds only `#[test]` fns, which it strips.

mod fixture;
mod harness;
mod rng;
mod words;
