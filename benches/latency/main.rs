//! `cargo bench -p chertmem` (benches I): every suite of `suites::ALL`
//! over its fixtures, under criterion. A first run with no
//! `OLLAMA_HOST` starts the stand-in ollama and runs again as a child,
//! so the semantic re-rank suite has a server (`ollama`).

mod fixture;
mod harness;
mod ollama;
mod ops;
mod rng;
mod suites;
mod words;

fn main() {
    let code = if ollama::needed() {
        harness::exit(ollama::again().and_then(ollama::serve_for))
    } else {
        let c = criterion::Criterion::default();
        let c = harness::configured(c, &harness::PUSH).configure_from_args();
        let out = harness::criterion_dir();
        harness::bench(c, &out, &suites::ALL, suites::grid)
    };
    std::process::exit(code);
}
