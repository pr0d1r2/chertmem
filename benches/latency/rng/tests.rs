// Only `#[test]` fns here, imports inside them: `cargo bench` builds
// this module with `--cfg test` and strips every test (lib.rs).

#[test]
fn the_same_seed_gives_the_same_stream() {
    use super::Rng;
    let (mut a, mut b) = (Rng::new(7), Rng::new(7));
    let left: Vec<u64> = (0..64).map(|_| a.next()).collect();
    let right: Vec<u64> = (0..64).map(|_| b.next()).collect();
    assert_eq!(left, right);
    assert_ne!(Rng::new(8).next(), Rng::new(7).next());
}

#[test]
fn splitmix64_matches_its_reference_output() {
    // The first output of the reference C code for seed 0.
    assert_eq!(super::Rng::new(0).next(), 0xe220_a839_7b1d_cdaf);
}

#[test]
fn below_stays_in_range_and_0_gives_0() {
    let mut r = super::Rng::new(1);
    assert!((0..1000).all(|_| r.below(10) < 10));
    assert_eq!(r.below(0), 0);
}

#[test]
fn skewed_favours_small_numbers() {
    let mut r = super::Rng::new(3);
    let draws: Vec<u64> = (0..10_000).map(|_| r.skewed(1000)).collect();
    let low = draws.iter().filter(|&&d| d < 100).count();
    let high = draws.iter().filter(|&&d| d >= 900).count();
    assert!(draws.iter().all(|&d| d < 1000));
    assert!(low > high.saturating_mul(10), "{low} low, {high} high");
}
