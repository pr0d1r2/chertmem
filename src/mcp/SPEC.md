# SPEC

## §G GOAL

`chert mcp`: MCP server on stdio w/ cavemem's 4 tools & shapes (D21); long-lived ∴ hosts `cache` (D14).

## §N NAV

rel|path|lens
up|.|-
up|src|crate `chertmem`: lib + bin `chert`, 1 node per module
self|src/mcp|MCP stdio server: 4 tools, hosts cache
sib|src/config|`chert.toml` scopes, merge, defaults, validation
sib|src/store|`catalog.db`, partition files, writes, spool, tiers, refs
sib|src/grammar|caveman compress & expand, ported from cavemem 0.2.1
sib|src/hook|hook intake: event JSON on stdin → redact → append
sib|src/cache|in-memory recent window in `chert mcp`: load, fit, freshness
sib|src/search|BM25 + vector ranking across partitions, one scoring path
sib|src/cli|`chert` verbs via clap, exit codes
sib|src/install|IDE wiring: hooks & MCP registration, status, doctor
sib|src/import|JSONL export for rekall, import incl. cavemem DB
sib|src/mine|behaviour findings & awareness report

## §C CONSTRAINTS

- transport stdio JSON-RPC. lib choice (`rmcp` vs hand-rolled) decided by T1 w/ §R row: binary size, deps, sync fit.
- results compressed unless `expand: true` (deviation from cavemem, stated in `.:§I`); observation `id` value = ref.
- logs → stderr only; ⊥ stdout noise.

## §I INTERFACES

- mcp: `search`, `timeline`, `get_observations`, `list_sessions` per `.:§I`.

## §R RESEARCH

id|topic|finding|src
R1|transport, wave 6|hand-rolled newline-delimited JSON-RPC 2.0 on `serde_json` (already a dep ∴ +0 crates, `Cargo.lock` unchanged): `initialize` `ping` `tools/list` `tools/call`, notifications & client responses ⊥ answered, else -32601; protocol version = asked if SDK 1.32.2 supports it (`2025-11-25` `2025-06-18` `2025-03-26` `2024-11-05` `2024-10-07`), else `2025-11-25`. `rmcp` 3.7.0 Apache-2.0 (license ok) but needs `tokio` `tokio-util` `futures` `tracing` `chrono` `thiserror` `indexmap` `pin-project-lite` ∴ async runtime vs `src:§C` sync rule ∴ rejected|`npm pack @modelcontextprotocol/sdk@1.32.2` `dist/esm/types.js`, `server/index.js`; `curl https://crates.io/api/v1/crates/rmcp/3.7.0/dependencies`
R2|tools, wave 6|entry `mcp::serve(&Ctx{config, root, env}, stdin, stdout)`; `env: cache::Env` carries log ∴ cli passes stderr. tool error → `{content, isError: true}` as SDK's `createToolError`. `search` scoped to current project (cache holds its window). `get_observations` adds `metadata: null` (⊥ stored); gone ref → `{id, dropped: true}`. 3 reading `Store`s: engine, cache, `get`|`server.rs`, `shape.rs`, cavemem `dist/server-*.js`
R3|`list_sessions`, wave 6→8|served from `Store::sessions(limit)` (src/store T16): current project, newest first. `limit` optional 1..=200, default 20, fields `{id, ide, cwd, started_at, ended_at}` ≡ cavemem; `ended_at` null while running. session w/ only an end seen (start missed) listed w/ `ide` `cwd` "" & `started_at` = its end, where cavemem ⊥ row|`server.rs`, `shape.rs`, cavemem `dist/server-*.js`, `src/store/session.rs`

## §V INVARIANTS

V1: tool names & field names ≡ cavemem 0.2.1 (`.:R1`); golden JSON test per tool.
V2: stdout carries protocol frames only.
V3: cache warnings ⊥ in tool results (`.:V11`).

## §T TASKS

id|status|task|cites
T1|x|transport lib choice, §R row|V2
T2|x|4 tools w/ golden JSON incl. `list_sessions` (R3)|V1
T3|x|start cache on launch|V3
T4|.|heartbeat for `chert statusline`: hold a file lock per running server & write median latency of last 20 replies to data dir after each reply, ⊥ slowing replies|V3

## §B BUGS

id|date|cause|fix
