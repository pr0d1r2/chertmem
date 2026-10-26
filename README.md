# chertmem

> Read the [LLM disclaimer] first: this
> repository was written mostly by an LLM, and that page says how it was
> built and what to check before trusting it.

Persistent memory for coding agents, in one Rust binary, `chert`.

chert records what happens in your coding agent's sessions (the prompts,
every tool call with its input and output, failed calls, turn and
session summaries) into a local store, and gives the agent a way to
search it later over MCP. Nothing leaves your machine unless you turn
on an embedding provider.

chert is a Rust rewrite of [cavemem](https://github.com/JuliusBrussee/cavemem)
0.2.1. It keeps cavemem's idea, its caveman compression grammar and its
four MCP tools with the same names and fields, and changes how the data
is kept:

- **No Node at run time.** cavemem starts Node for every hook event;
  chert is a single native binary. Writing one hook event, measured on
  an Apple M1 Pro on 2026-10-25, took a median of 4.51 ms of wall clock
  for the whole process (50 runs; how it was measured is in
  `src/cli/SPEC.md`, R4).
- **A partitioned store.** One SQLite file per project and time bucket
  instead of one file for everything, so a search reads only the
  partitions that can hold an answer.
- **A cache that is never stale.** The long-lived MCP server keeps the
  recent weeks in memory and checks the store before every query.
- **Short, stable references.** Every observation has a `chert:<id>` ref
  that keeps pointing at the same row.
- **Behaviour mining.** `chert mine` finds repeated failures, repeated
  command sequences, hot files and token sinks, each with the refs that
  prove it.

## Status

Early: 0.1.0 is the first release ([CHANGELOG](CHANGELOG.md)), and the
interface may still change before 1.0. The one IDE wired in v1 is [Claude Code](https://claude.com/claude-code):
other IDEs follow once their hook payloads are recorded. Not built yet:
`chert export --for rekall` (the plain export works) and
`embedding.provider = "openai"` (search falls back to keywords).

Exit status: 0 ok, 1 a finding, 2 usage or unreadable input. Every
message names its fix.

## Install

From crates.io, with Rust 1.95 or newer:

```sh
cargo install chertmem --locked
```

Or from a checkout:

```sh
git clone https://github.com/pr0d1r2/chertmem
cd chertmem
cargo install --path . --locked
```

That puts `chert` in `~/.cargo/bin`. SQLite and zstd are compiled into
the binary, so it needs nothing else on `PATH`.

## Quick start

```sh
chert install    # wire the hooks and the MCP server into Claude Code
chert status     # every entry chert wrote, and whether it is in place
```

`chert install` writes its six hooks into `~/.claude/settings.json` and
the `chert` MCP server into `~/.claude.json`, keeping every other key
and making a `.chert.bak` backup of a file before its first change. Run
it from the `chert` you will keep: the hooks call that binary by its
full path. Running it twice changes nothing the second time.

```text
$ chert status
hook SessionStart: ok
hook UserPromptSubmit: ok
hook PostToolUse: ok
hook PostToolUseFailure: ok
hook Stop: ok
hook SessionEnd: ok
mcp server chert: ok
```

Then use Claude Code as usual. The agent can call the MCP tools itself;
you can ask the same questions from a shell, inside the project:

```sh
chert search echo --limit 3           # ranked hits: ref, score, session, time in ms, snippet
chert get chert:01M5RQN8              # the full observation behind a ref
chert timeline <session-id>           # a session's rows in order
chert mine                            # behaviour findings, with the refs as evidence
chert report --format md              # every finding as a Markdown table
```

A hit looks like this, the ref first:

```text
$ chert search echo --limit 3
chert:01M5RQN81 0.8127 00000000-0000-4000-8000-000000000001 1792905224253 d reply with its output only: echo chert-fixture
...
```

`chert mine` exits 1 when it has findings, so it can gate a script:

```text
$ chert mine
4x failure 3813138cdc2d5cd0 failing `Bash` `false`: `Exit code 1`
  chert:01M5RR2K chert:01M5RR2K1 chert:01M5RR2K2 chert:01M5RR2K2C
```

`chert doctor` checks the wiring, the catalog against the partition
files, and spooled events that were never stored; `chert uninstall`
removes only what `chert install` wrote.

## What is stored, and what is not

Each hook event becomes one observation: the prompt, a tool call's name,
input and output, a failed call's error, a turn summary taken from the
agent's last message, and a session summary made of the first 20 turn
summaries. No model is involved.

Before anything is written, chert removes every `<private>…</private>`
span from the text and skips any tool call naming a path in
`privacy.exclude`. Neither reaches the store, the cache or an export.
The rest is compressed: first by the caveman grammar (`I think that the
configuration of the database is basically really important` becomes
`config of db is important`), then by zstd when a partition goes cold.
Code, paths, URLs, identifiers and numbers are never rewritten.

## How the store is partitioned

The data lives under `~/.local/share/chert` (`storage.data_dir`):

```text
catalog.db                                  one row per partition: bounds, rows, bytes, tier, checksum
chunks/<project>/<bucket>[.<shard>].db      one SQLite file per partition
chunks/<project>/<bucket>[.<shard>].db.zst  a cold partition, zstd-packed
spool/                                      events that could not be written yet
```

A project is named after its directory plus a hash of its path, so two
checkouts with the same name stay apart. By default each partition holds
one project's week (`partition.by = ["project", "time"]`,
`partition.time_bucket = "7d"`); both are settings, and `"ide"` can be
added as a dimension. A search asks `catalog.db` which partitions
overlap the query and opens only those, one SQLite connection each; it
reads the bounds a partition was written under, so changing the settings
later never hides old rows.

Partitions move through three tiers: hot (taking writes), warm (sealed,
read-only) and cold (zstd-packed, with a trained dictionary by default). See them
with `chert partitions`, and move them with `chert partitions seal`,
`compact`, `drop` and `repartition`. Seal or compact a bucket only once
its time has passed: an event for a sealed bucket is set aside in the
spool, not stored.

Many processes use the store at once (every hook, every agent session's
MCP server), so every file runs in SQLite's WAL mode. A hook that cannot
get the write lock in time puts its event in the spool and exits; the
next reader stores it before reading.

## The cache

`chert mcp`, the server Claude Code starts for each session, keeps the
current project's recent window in memory: the last 28 days by default
(`cache.window`), up to 256 MiB (`cache.max_bytes`). Before every query
it checks whether another process has written, deleted or rewritten
anything, and catches up first, so an answer from the cache is the same
as an answer from disk: same rows, same order, same scores. When the
window would not fit, chert keeps the newest days that do and says so
(`cache.on_overflow`). `chert cache plan` prints the estimate:

```text
$ chert cache plan
estimate 6949 bytes, limit 268435456 bytes, 28 days kept
```

## `chert:<id>` refs

Every observation gets a ref when it is stored: `chert:` and the
shortest prefix of its ULID, at least 8 characters, that no earlier ref
in the project has used. The ref is stored, not recomputed, and looked
up by exact match, so later rows never make it ambiguous. It keeps
pointing at the same observation across sealing, compaction and
repartitioning. When its partition is dropped, the ref answers
`dropped` (and `chert get` exits 1), never another row.

Refs are what the MCP tools hand the agent, what the session-start
context lists, and what `chert mine` cites as evidence, so a note or a
skill written from chert's memory can point back at the exact event.

## Documentation

- [Integration]: wiring chert into Claude Code, the MCP tools,
  importing from cavemem, and exporting.
- [Configuration]: every `chert.toml` key, its default and its meaning.
- [Benchmarks]: how chert's speed is measured, the targets and why;
  [Performance]: the latest measured numbers, generated from the
  baseline.
- `chert --help` and `chert man`: every command and flag;
  `chert completions <shell>` prints shell completions.
- [Contributing], [security], the [code of conduct], the
  [LLM disclaimer] and the
  [third-party notices](THIRD-PARTY-NOTICES.md).

The design lives in [`SPEC.md`][spec] and one `SPEC.md` per node under
`src/`, `benches/`, `dev/` and `docs/`.

<!-- Absolute links: these pages are not in the crates.io package. -->

[Integration]: https://github.com/pr0d1r2/chertmem/blob/main/docs/INTEGRATION.md
[Configuration]: https://github.com/pr0d1r2/chertmem/blob/main/docs/CONFIG.md
[Benchmarks]: https://github.com/pr0d1r2/chertmem/blob/main/docs/BENCHMARKS.md
[Performance]: https://github.com/pr0d1r2/chertmem/blob/main/docs/PERFORMANCE.md
[Contributing]: https://github.com/pr0d1r2/chertmem/blob/main/CONTRIBUTING.md
[security]: https://github.com/pr0d1r2/chertmem/blob/main/SECURITY.md
[code of conduct]: https://github.com/pr0d1r2/chertmem/blob/main/CODE_OF_CONDUCT.md
[LLM disclaimer]: https://github.com/pr0d1r2/chertmem/blob/main/docs/LLM-DISCLAIMER.md
[spec]: https://github.com/pr0d1r2/chertmem/blob/main/SPEC.md

## License

[MIT](LICENSE)
