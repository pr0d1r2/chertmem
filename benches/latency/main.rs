//! `cargo bench -p chertmem` (benches I): criterion, configured for a
//! push; the suites follow (benches T3).

mod harness;

fn main() {
    let c = criterion::Criterion::default();
    let c = harness::configured(c, &harness::PUSH).configure_from_args();
    c.final_summary();
}
