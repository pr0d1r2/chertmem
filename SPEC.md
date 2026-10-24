# SPEC

## §G GOAL

`chertmem` — persistent memory for coding agents, 1 static Rust binary `chert`, ⊥ Node at run time. hooks capture session events → partitioned local store (SQLite file per project × time bucket, TimescaleDB-style chunk exclusion) keeps them compressed (caveman grammar + zstd) → agents retrieve targeted rows over MCP, fast & correct: cached answer ≡ uncached answer. also: awareness report & behaviour mining, exported for rekall; extracted skills point back by short `chert:<id>` refs.

LINEAGE: `SPEC-PROMPT.md` (HISTORICAL after this run). where it & this spec differ, this spec wins. prompt decisions cited `D<n>`. paradigm `fda10ea`, no Rust language page yet; fleet rules cited `paradigm/V<n>`.

## §D DESCRIPTION

Rust cargo workspace: crate `chertmem` (lib + bin `chert`, MIT, public) & `dev` (`publish = false`, gate steps). rewrite of cavemem 0.2.1 (npm, `JuliusBrussee/cavemem`, MIT): same capture idea & MCP tool names, own data dir, import path from cavemem. store = `catalog.db` + 1 SQLite file per partition; long-lived `chert mcp` holds bounded in-memory cache of recent window. hooks = short processes, write & exit. mining reports behaviour findings; rekall reads chert's JSONL export & alone mints candidates (D27). fleet member, full paradigm conformance (D10).

## §F FEDERATION

dir|owns|⊥owns|tokens

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
