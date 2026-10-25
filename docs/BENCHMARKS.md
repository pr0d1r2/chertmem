# Benchmarks

chert has four latency targets, and a benchmark suite measures each of
them on generated stores of fixed sizes. The latest numbers, with the
machine, the date and the command that measures each one again, are on
[Performance](PERFORMANCE.md). This page says how the benchmarks run,
how to read them, and how a slowdown is caught before it is pushed. It
holds no measured numbers of its own, so it never goes stale.

## The targets, and why

| Target | Under | Why |
| --- | --- | --- |
| hook write | 5 ms | Every hook runs while the agent waits for it: on each tool call, prompt and stop. A slow hook is felt as a slow agent. |
| recent-window search | 10 ms | The search an agent makes most: the last weeks of one project, served from the cache `chert mcp` keeps warm. |
| `get_observations` | 2 ms | After a search the agent fetches the few rows it picked, by reference; that should cost nothing it notices. |
| semantic re-rank | 20 ms | The opt-in embedding pass over a search's hits. It adds a round trip to an embedding server, so it gets a wider budget. |

The targets come from the project spec (`SPEC.md`, its constraints),
and the performance page reads them from there, so the two cannot
disagree. When the targets and speed conflict with correctness, chert
picks the correct answer: a faster answer that differs from the
uncached one is a bug, not a win.

## The suites

Each suite is one criterion group, named as on the performance page.

| Suite | What one call does | Target |
| --- | --- | --- |
| `session_start` | the `session-start` hook in process: write the event, then build the injected context | hook write |
| `prompt_inject` | the `user-prompt-submit` hook with `inject.on_prompt = true`: write, then inject | hook write |
| `get_observations` | resolve five references from the newest week to their rows | `get_observations` |
| `recent_warm` | search the cache window (28 days) through a warm cache, as `chert mcp` does | recent-window search |
| `recent_cold` | the same search read from disk, no cache | recent-window search |
| `semantic_rerank` | a search re-ranked by embeddings from a local stand-in server | semantic re-rank |
| `cross_partition` | a search over all time, every partition | none |
| `cache_warmup` | start a cache and wait until its window is loaded and indexed | none |
| `partition_scan` | read the newest weekly partition whole | none |
| `compress`, `expand` | the caveman grammar over the bodies of the smallest store | none |

The hook-write suite proper, which starts a real `chert hook run`
process so its start-up time counts, is listed by the bench run as
pending until the command line is wired. The two injecting suites run
the hook in process: they measure chert's work without the process
start.

The semantic suite never calls a real model. The bench starts a small
embedding server on the loopback interface and points `OLLAMA_HOST` at
it, so it measures chert's side of the call and nothing else.

## The stores they run on

The store suites run on generated stores, built from a fixed seed by a
generator in `benches/latency/fixture`, so the same seed gives the same
rows on every machine:

- 1 000, 10 000, 100 000 and 1 000 000 observations over a year of
  weekly partitions (52), and
- 100 000 observations over 1, 4 and 520 partitions, to show what the
  partition count costs.

A fixture is labelled by size and partitions: `100k-p520` is 100 000
observations over 520 weeks. The sessions look like Claude Code ones:
a session start, tool calls, prompts, stops, failures and a session
end with its summary, bodies compressed as the hooks store them.

Fixtures are built once and cached under `target/bench-fixtures/`. The
largest is slow to build and large on disk, so the first run is
slow and later ones are not. A change to the generator changes its
version, and stale fixtures are rebuilt on the next run.

## Running them

```sh
cargo bench -p chertmem --bench latency                    # every suite, every store
cargo bench -p chertmem --bench latency -- '^recent_warm/'  # one suite
cargo bench -p chertmem --bench latency -- '/1k-p52$'      # one store size
```

The filter after `--` is criterion's: a regular expression over
`<suite>/<fixture>`. Each bench takes 10 samples after a short warm-up,
so a run is quick once its fixtures are cached.

## Reading the output

criterion prints three numbers per bench:

```text
get_observations/1k-p52   time:   [low  median  high]
```

The middle one is the median time of one call; the outer two are the
bounds of its confidence interval. The baseline and the performance
page use the median, and the 95th percentile of the samples to show
the slow tail. criterion's own `change:` line compares with its
previous run on the same machine; the gate below does not use it.

The raw numbers are under `target/criterion/<suite>/<fixture>/new/`:
`estimates.json` for the median and `sample.json` for the samples.

## The baseline

`benches/baseline.toml` holds one row per suite and fixture: the
median, the 95th percentile, the target, a tolerance, the machine
(CPU model and OS), the date the numbers were measured and the command
that measures that row again. It is written by a tool, never by hand,
except for the tolerance:

```sh
cargo bench -p chertmem --bench latency          # a full run, on a quiet machine
cargo run -q -p chertmem-dev -- bench-record     # target/criterion into benches/baseline.toml
cargo run -q -p chertmem-dev -- perf-doc         # benches/baseline.toml into docs/PERFORMANCE.md
```

A record replaces the rows it measured on the same machine class and
keeps every other row, so a partial run re-records only itself and
rows from other machines stay. A tolerance tuned by hand in the file
survives the next record.

The baseline changes only in its own commit, with the page it
generates, so a reviewer sees the numbers move on their own and asks
why. The commit gate checks that `docs/PERFORMANCE.md` is exactly what
`perf-doc` makes from the baseline and the targets.

## The regression rule

A bench has regressed when its median now is over its baseline median
times one plus the tolerance, measured on the same machine class: the
same CPU model and the same OS, whatever its version. Rows from other
machines are not compared, since a different CPU is not a slowdown.

```sh
cargo run -q -p chertmem-dev -- bench-gate                 # every baseline row with a fresh median
cargo run -q -p chertmem-dev -- bench-gate 1k-p52 10k-p52  # only these stores
```

The gate reads criterion's last run, prints each regression with the
row's recount command, and exits 1 when there is one. It exits 0 with
a note on a machine the baseline has no rows for.

Before each push, the gate step runs the store suites on the two
smallest stores, `1k-p52` and `10k-p52`, then the gate over them. That
keeps the push short once the release build is cached, while every
store suite still runs. The larger stores are slow to build and to
measure, so they run by hand before a baseline is recorded again. CI
has no baseline rows for its machines yet, so there the gate would hold
without comparing anything; it runs where the baseline was measured.
