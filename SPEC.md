# SPEC

## §G GOAL

`chertmem` — persistent memory for coding agents, 1 static Rust binary `chert`, ⊥ Node at run time. hooks capture session events → partitioned local store (SQLite file per project × time bucket, TimescaleDB-style chunk exclusion) keeps them compressed (caveman grammar + zstd) → agents retrieve targeted rows over MCP, fast & correct: cached answer ≡ uncached answer. also: awareness report & behaviour mining, exported for rekall; extracted skills point back by short `chert:<id>` refs.

LINEAGE: `SPEC-PROMPT.md` (HISTORICAL after this run). where it & this spec differ, this spec wins. prompt decisions cited `D<n>`. paradigm `fda10ea`, no Rust language page yet; fleet rules cited `paradigm/V<n>`.

## §D DESCRIPTION

Rust cargo workspace: crate `chertmem` (lib + bin `chert`, MIT, public) & `dev` (`publish = false`, gate steps). rewrite of cavemem 0.2.1 (npm, `JuliusBrussee/cavemem`, MIT): same capture idea & MCP tool names, own data dir, import path from cavemem. store = `catalog.db` + 1 SQLite file per partition; long-lived `chert mcp` holds bounded in-memory cache of recent window. hooks = short processes, write & exit. mining reports behaviour findings; rekall reads chert's JSONL export & alone mints candidates (D27). fleet member, full paradigm conformance (D10).

## §F FEDERATION

dir|owns|⊥owns|tokens
src|crate `chertmem`: lib + bin `chert`, 1 node per module|gate steps (`dev`), benchmarks (`benches`), human pages (`docs`)|-
benches|benchmarks: fixtures, suites, baseline, regression gate|product code (`src`), the human page (`docs`)|-
dev|dev shell flake & `dev` crate (`publish = false`) gate steps|product code (`src`), hk steps (`.pklith`)|-

## §N NAV

rel|path|lens
up|-|-
self|.|-

## §C CONSTRAINTS

- Rust, `unsafe` forbidden. edition, MSRV, clippy, rustfmt, cargo-deny, ASCII-only source, `rust-mirror` test layout = fleet values read from public members (`paradigm/V42`), ⊥ typed here. clap for args (`paradigm/V29`).
- 1 binary `chert`, ⊥ Node | runtime on PATH at run time; ⊥ native addon tied to a runtime ABI (D1). SQLite via `rusqlite` `bundled` (FTS5 compiled in). Node = test-only dep of grammar oracle (D22).
- priority when they conflict: correctness > responsiveness (D3). ⊥ faster answer that is less correct.
- ⊥ required server | daemon; ⊥ network by default. only opt-in egress: `embedding.provider = ollama|openai` (D18).
- store on disk = source of truth; cache = read accelerator only (D8, D14).
- many processes at once: hooks, ∀ IDE session's `chert mcp`, worker ∴ store ! multi-process safe (SQLite WAL); single-process stores rejected.
- config `chert.toml`: user `~/.config/chert/chert.toml` + project `chert.toml`, project overrides key by key. ∀ partition & cache setting configurable (D5, D6, D7).
- defaults (D19): `partition.by = ["project","time"]`, `time_bucket = "7d"`, `shards = 1`, `cache.window = "28d"`, `cache.max_bytes = "256MiB"`, `on_overflow = "warn"`, `embedding.provider = "off"`.
- latency targets (D20), benchmark-gated: hook write < 5 ms · recent-window search < 10 ms · `get_observations` < 2 ms · semantic re-rank < 20 ms.
- `<private>…</private>` stripped & `privacy.exclude` paths ⊥ read, before any write.
- chert ⊥ writes `.rekall/` (D27). mining ⊥ changes anything by itself.
- cavemem: own data dir; ⊥ share cavemem's live files (D21). cavemem MIT notice ships w/ grammar port (D22).
- IDEs v1: Claude Code. others once hook-payload fixtures recorded (D24). ⊥ web viewer v1 (D25).
- public on GitHub from first push, MIT; CI = GitHub Actions rendered by actlith (D26, `paradigm/V28`). systems per `paradigm/V28`.
- dev shell per `paradigm/V20`; gate per `paradigm/V23`; atomic commits, 1 topic each, `Paradigm-Stage:` trailer (`paradigm/V36`, D10).
- outward acts need owner approval: push, publish, issues in other repos (rekall `chert` dialect, D27).

## §I INTERFACES

- cmd: `chert hook run <event>` — event JSON on stdin → 1 write → exit. events: `session-start` `user-prompt-submit` `post-tool-use` `stop` `session-end`.
- cmd: `chert mcp` — MCP server on stdio; hosts cache (D14).
- mcp: `search(query, limit?)` → `[{ref, score, snippet, session_id, ts}]`; `timeline(session_id, around?, limit?)` → `[{ref, kind, ts}]`; `get_observations(refs[], expand?)` → bodies, compressed unless `expand`; `list_sessions(limit?)` → `[{id, ide, cwd, started_at, ended_at}]`. names & shapes kept from cavemem (D21); `id` field carries ref.
- ref: `chert:<id>` — shortest ULID prefix unique within project, ≥ 8 chars (D28). gone observation → answer `dropped`, ⊥ other row.
- cmd: `chert search <query> [--limit N] [--since T] [--project P] [--no-semantic]`; `chert timeline <session> [--around REF]`; `chert get <ref>...`.
- cmd: `chert install|uninstall [--ide NAME]`; `chert status`; `chert doctor`; `chert config show|get|set|path`.
- cmd: `chert partitions [list|seal|compact|drop|repartition]`; `chert cache [plan]`.
- cmd: `chert mine [--since T] [--format text|json]`; `chert report [--format text|json|md]`.
- cmd: `chert compress <file>`; `chert expand <file>`; `chert export <out.jsonl> [--for rekall]`; `chert import <in.jsonl> [--from cavemem]`; `chert completions <shell>`; `chert man`.
- exit: 0 ok · 1 finding · 2 usage | unreadable input.
- file: `chert.toml` — user & project scope (§C).
- dir: data dir (default `~/.local/share/chert`): `catalog.db`, `chunks/<project>/<bucket>[.<shard>].db[.zst]`, `spool/`.
- export: JSONL, 1 observation per line, `ref` field; read by rekall `catch` via `chert` dialect (D27).
- env: `OLLAMA_HOST` | provider key, only when `embedding.provider` opts in.

## §R RESEARCH

id|topic|finding|src
R1|cavemem 0.2.1 2026-10-24|MIT, 676 stars, Node ≥ 20. 6 deps: `commander` `kleur` `better-sqlite3` (native addon) `hono` `@hono/node-server` `@modelcontextprotocol/sdk`; optional `@xenova/transformers`. 5 hook shims `exec cavemem hook run <name>` ∴ 1 Node start per event. 1 SQLite file: `sessions` `observations` `summaries` `embeddings` `schema_version` + FTS5 `observations_fts` w/ 3 triggers. 4 MCP tools. bundle 214573 B, 160478 in shared chunk (grammar, store, ranker)|`npm pack cavemem@0.2.1`; grep `dist/*.js`; `gh api repos/JuliusBrussee/cavemem`
R2|Rust agent-memory 2026-10-24|leteo 4★ · engram-mcp 18★ · opencode-memory 1★ · mcpmem 0★ · memory-engine 2★ · agentos 1★. all created 2026, ⊥ partitioned store, ⊥ caveman grammar, ⊥ cavemem data ∴ reference code only|`gh api repos/<repo>`; `curl https://crates.io/api/v1/crates/<crate>`
R3|rekall 3235597|candidate = `{id, src, text, at}` in `.rekall/ledger.toml`; only rekall's classifier mints (rekall `src/catch` rule 44); ⊥ 2nd candidate store (rekall `src/catch` rule 45); `catch` reads transcript roots, dialect per agent (rekall `src/catch` rule 47)|`github.com/pr0d1r2/rekall` `src/ledger/mod.rs`, `src/catch/SPEC.md`
R4|names 2026-10-24|`chertmem` free on crates.io & npm, 0 GitHub repos; `chert` crate = `jesopo/chert` 0.2.1 (2024-02-06); ⊥ Homebrew formula `chert`|`curl https://crates.io/api/v1/crates/<name>`; `gh search repos chertmem`; `brew info --formula chert`
R5|SQLite limits|`ATTACH` default max 10 dbs (`SQLITE_MAX_ATTACHED`) ∴ 1 connection per partition, ⊥ attach-all. WAL: readers ⊥ block writer, 1 writer at a time, multi-process|`https://sqlite.org/limits.html`; `https://sqlite.org/wal.html`

## §V INVARIANTS

V1: ∀ query: cached answer ≡ uncached answer — same rows, same order, same scores (D8). ∵ cache = accelerator ⊥ second truth.
V2: ∀ accepted event → in partition | in spool, ⊥ lost; spool replayed next run (D13). ∵ hook ⊥ retry.
V3: write idempotent: same (session, event, payload) twice → 1 row (D13). ∵ IDEs resend events.
V4: planner routes by `catalog.db` bounds, ⊥ by current config ∴ config change ⊥ misroutes read (D12).
V5: ref `chert:<id>` resolves to 1 observation ∀ its life, across repartition & compaction; gone → `dropped`, ⊥ other row (D28).
V6: ⊥ network unless `embedding.provider` opts in; ⊥ other egress at any flag (§C).
V7: chert ⊥ writes under `.rekall/`; rekall reads chert's export (D27).
V8: `<private>…</private>` stripped & `privacy.exclude` paths ⊥ read before any write ∴ ⊥ byte of either reaches store, cache, export.
V9: compress, expand, mine, report deterministic: same input & version → same bytes (D22, D23).
V10: §C latency targets measured by benchmark step on push; regression fails gate (D20).
V11: cache estimate > `cache.max_bytes` → warning names estimate & limit (D7); warnings → MCP log, `status`, `doctor`, ⊥ agent context (D16).
V12: ⊥ private repo named in committed text (`paradigm/V37`).
V13: ∀ outward act (push, publish, issue in other repo) has owner approval on record.
V14: ∀ §C latency target, cache warm-up & partition scan at stated sizes: 1 benchmark on generated fixtures of stated size; `docs/BENCHMARKS.md` explains run, read, targets & why, & ∀ measured number w/ date, machine, recount command (D29, `paradigm/V4`).

## §T TASKS

id|status|task|cites
T1|x|vendor `FORMAT.md` & `FORMAT-EXTENSIONS.md` from paradigm `fda10ea`|-
T2|~|spec run: root sections & ∀ node, 1 per commit, trailer `spec-run`|-
T3|.|freeze `SPEC-PROMPT.md`: line 1 → `HISTORICAL INPUT`, own commit|-
T4|.|`.context-limits` from `sherd budget` + 12–20%|-
T5|x|`.gitignore`: `.claude/`, `.direnv/`, `/target`, `result*`, `.envrc.local`, `.private/` (first commit)|V12
T6|.|dev shell (`dev`): pinned toolchain, `mth` `sherd` `itok` on PATH; ask human to reload|-
T7|.|guardrails via `.pklith`, 1 check per commit, planted violation each; ask human to reload|V10,V12
T8|.|waves via `sherd wave`, order: `src/config` → `src/store` → `src/grammar` → `src/hook` → `src/cache` → `src/search` → `src/mcp` → `src/cli` → `src/install` → `src/import` → `src/mine` → `benches` → `docs`|V1,V2,V3,V4,V5
T9|.|grammar oracle: corpus + cavemem 0.2.1 JavaScript pinned by hash, test-only|V9
T10|.|benchmark step on push w/ baseline file; regression fails|V10,V14
T11|.|defaults (D19) checked against 1 real month of history; record in §R|V11
T12|.|CI via actlith: gate on `paradigm/V28` systems|V10
T13|.|GitHub repo `pr0d1r2/chertmem` PUBLIC, first push, w/ approval; `gh repo view` ∀ named repo first|V12,V13
T14|.|issue in rekall proposing `chert` dialect for `catch`, w/ approval (D27)|V7,V13
T15|.|owner review of agent-answered round 2 (D11–D26)|-
T16|.|`docs/BENCHMARKS.md`: human page on benchmarks, first measured numbers|V14

## §B BUGS

id|date|cause|fix
