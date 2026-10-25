# SPEC

## §G GOAL

clap front end: parse ∀ `.:§I` verb, call owning module, map result to exit 0 | 1 | 2.

## §N NAV

rel|path|lens
up|.|-
up|src|crate `chertmem`: lib + bin `chert`, 1 node per module
self|src/cli|`chert` verbs via clap, exit codes
sib|src/config|`chert.toml` scopes, merge, defaults, validation
sib|src/store|`catalog.db`, partition files, writes, spool, tiers, refs
sib|src/grammar|caveman compress & expand, ported from cavemem 0.2.1
sib|src/hook|hook intake: event JSON on stdin → redact → append
sib|src/cache|in-memory recent window in `chert mcp`: load, fit, freshness
sib|src/search|BM25 + vector ranking across partitions, one scoring path
sib|src/mcp|MCP stdio server: 4 tools, hosts cache
sib|src/install|IDE wiring: hooks & MCP registration, status, doctor
sib|src/import|JSONL export for rekall, import incl. cavemem DB
sib|src/mine|behaviour findings & awareness report

## §C CONSTRAINTS

- clap builder API (`paradigm/V29`), ⊥ derive: its proc-macro tree brings unicode-ident (Unicode-3.0) into the build; owner 2026-10-25 "take MIT if possible" (R5). `completions` & `man` generated from same `clap::Command`.
- ⊥ logic beyond dispatch & output formatting.

## §I INTERFACES

- fn: `cli::run(args, &mut Io) -> u8` (R3); `cli::main()` = system entry.

## §R RESEARCH

id|topic|finding|src
R1|hook exit, wave 8|Claude Code reads a hook's exit 2 as blocking: stderr goes back to the agent & `UserPromptSubmit` drops the prompt ∴ `hook run` exits as `hook::run` says (0, `src/hook:V2`) & a config or stdin it cannot read → stderr note, exit 0, ⊥ 2. only clap refusals (`hook run <unknown>`) exit 2|Claude Code hooks docs, exit code 2 behaviour
R2|exit map, wave 8|1 = finding: `mine` w/ findings, `get` naming a dropped ref, `timeline --around` a dropped ref, `cache plan` estimate > `cache.max_bytes`, `doctor` w/ findings (`install::Report::exit`). 0: `report`, `search` & `timeline` w/ any rows incl. none, `partitions` verbs, `compress`, `expand`, `export`, `import`, `mcp` on EOF, `completions`, `man`, `install`, `uninstall`, `status`, `config`. 2: library errors, unreadable files, ref ⊥ `chert:<ref>`, clap errors, `export --for rekall` (`src/import` T3)|`src/cli/*/tests.rs`
R3|as built, wave 8|fn = `cli::run(args, &mut Io) -> u8`, `Io` = stdin, stdout, stderr, cwd (project root), `home: Option<&Path>`, `now`, wired `chert` path; `cli::main` = system entry, `main.rs` a shim. `--since` = `<n>h`, `<n>d` or `<n>mo` back from now (month = 31 d, as cache) or raw ms. `export` writes whole store (`Filter::default`); `compress` uses `compression.grammar`; `config set` writes project file, `--user` user file; `cache plan` starts cache w/ silent log & reads its plan; `search --project` takes id as `partitions list` prints|`src/cli.rs`, `src/cli/`
R4|hook latency, wave 8|`chert hook run post-tool-use < tests/hook/fixtures/claude-code/post-tool-use.json`, release, temp HOME & project, fork+exec+wait wall clock, 50 runs: median 4.51 ms, p95 4.84 ms (`/usr/bin/true` same harness: median 1.82 ms) vs root §C < 5 ms. Apple M1 Pro, macOS 26.6.2, 2026-10-25|perl `Time::HiRes` loop over fork, exec, waitpid
R5|builder, wave 11|derive → builder: help ∀ verb, errors, completions (5 shells) & man byte-identical; only derive's implicit unrequired ArgGroups gone. `cargo tree -e normal,build -i unicode-ident` empty; dev graph keeps it via criterion → serde_derive ∴ deny.toml allows Unicode-3.0 for unicode-ident alone. test: `cargo metadata` walk, 4 deny targets|`src/cli/args/tests.rs`, `deny.toml`

## §V INVARIANTS

V1: help text, completions & man page generated from 1 clap definition ∴ ⊥ drift.
V2: exit 0 ok · 1 finding · 2 usage | unreadable input (`.:§I`); test per verb.

## §T TASKS

id|status|task|cites
T1|x|clap definition ∀ verbs|V1
T2|x|exit-code tests|V2
T3|.|`chert statusline` (root §I): ms from mcp heartbeat, n & size (B·KB·MB·GB, rounded up) from catalog; ANSI per value, thresholds in `[statusline]`: ms 10/50, n by cache fit 80%, size 1/5 GB; label red ⊥ live mcp; < 5 ms|V1,V2

## §B BUGS

id|date|cause|fix
