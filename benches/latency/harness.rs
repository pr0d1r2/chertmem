//! The harness, criterion (benches T1, R1): how long each bench runs,
//! and every ready suite registered over its grid of fixtures.

#[cfg(test)]
mod tests;

use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::time::Duration;

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion};

use crate::fixture::{self, Spec};
use crate::suites::{Make, Op, Status, Suite};

/// How long criterion samples 1 bench.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Timing {
    pub(crate) samples: usize,
    pub(crate) warm_up: Duration,
    pub(crate) measure: Duration,
}

/// On push (root V10): criterion's least sample count, short windows,
/// so a bench costs about 1.3 s after its fixture is built.
pub(crate) const PUSH: Timing =
    Timing::new(10, Duration::from_millis(300), Duration::from_secs(1));

impl Timing {
    pub(crate) const fn new(
        samples: usize,
        warm_up: Duration,
        measure: Duration,
    ) -> Self {
        Self {
            samples,
            warm_up,
            measure,
        }
    }
}

// Criterion refuses fewer than 10 samples.
const _: () = assert!(PUSH.samples >= 10);

/// `c` sampling as `t` says, without plots.
pub(crate) fn configured(c: Criterion, t: &Timing) -> Criterion {
    c.sample_size(t.samples)
        .warm_up_time(t.warm_up)
        .measurement_time(t.measure)
        .without_plots()
}

/// Every ready suite as 1 criterion group over `grid(suite)`, then the
/// summary; a pending one is named with what it waits for. The exit
/// code, 0: a bench that fails ends the process first ([`fail`]).
pub(crate) fn run(
    mut c: Criterion,
    suites: &[Suite],
    grid: fn(&Suite) -> Vec<Spec>,
) -> i32 {
    for suite in suites {
        match suite.status {
            Status::Pending(why) => eprintln!("{}: pending, {why}", suite.id),
            Status::Ready(make) => {
                eprintln!("{}: {}", suite.id, measures(suite));
                group(&mut c, suite.id, make, &grid(suite));
            }
        }
    }
    c.final_summary();
    0
}

/// The process's exit code for `done`: an error is printed, then 2.
pub(crate) fn exit(done: fixture::Res<i32>) -> i32 {
    done.unwrap_or_else(|e| {
        eprintln!("chert bench: {e}");
        2
    })
}

/// What `suite` measures, against which root C target.
pub(crate) fn measures(suite: &Suite) -> String {
    match suite.target {
        Some(t) => format!("{}, target {} < {} ms", suite.spec, t.name, t.ms),
        None => format!("{}, no root C target", suite.spec),
    }
}

fn group(c: &mut Criterion, id: &str, make: Make, specs: &[Spec]) {
    let mut g = c.benchmark_group(id);
    for spec in specs {
        one(&mut g, make, *spec);
    }
    g.finish();
}

/// 1 bench; its fixture and setup made on first call, so a filtered
/// out bench builds nothing.
fn one(g: &mut BenchmarkGroup<'_, WallTime>, make: Make, spec: Spec) {
    let mut op: Option<Op> = None;
    g.bench_function(spec.label(), |b| {
        let op = op.get_or_insert_with(|| setup(make, spec));
        b.iter(|| op().unwrap_or_else(|e| fail(&e)));
    });
}

fn setup(make: Make, spec: Spec) -> Op {
    let fx = fixture::cached(spec).unwrap_or_else(|e| fail(&e));
    make(&fx).unwrap_or_else(|e| fail(&e))
}

/// A bench that cannot run stops the whole run: no number beats a
/// wrong one (root C).
fn fail(e: &dyn Display) -> ! {
    eprintln!("chert bench: {e}");
    std::process::exit(1)
}

/// [`run`] with its output in `out`, the suite index written there
/// first; 2 when it cannot be.
pub(crate) fn bench(
    c: Criterion,
    out: &Path,
    suites: &[Suite],
    grid: fn(&Suite) -> Vec<Spec>,
) -> i32 {
    if let Err(e) = write_index(out, suites) {
        eprintln!("chert bench: cannot write the suite index: {e}");
        return 2;
    }
    run(c.output_directory(out), suites, grid)
}

/// The suite index's file name, beside criterion's group dirs.
pub(crate) const INDEX: &str = "chert-suites.tsv";

/// Where criterion writes: beside the fixtures, `target/criterion`.
pub(crate) fn criterion_dir() -> PathBuf {
    fixture::base().with_file_name("criterion")
}

/// The suite index `chertmem-dev bench-record` reads (benches T4): 1
/// line per ready suite, `<id>\t<root C target>`, `-` for none.
pub(crate) fn index(suites: &[Suite]) -> String {
    let _ = suites;
    String::new()
}

/// [`index`] written to `dir`/[`INDEX`].
pub(crate) fn write_index(dir: &Path, suites: &[Suite]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join(INDEX), index(suites))
}
