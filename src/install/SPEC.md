# SPEC

## §G GOAL

`chert install|uninstall [--ide NAME]`: register hook shims & MCP server in IDE config; `status` & `doctor` report wiring. v1 IDE = Claude Code (D24).

## §N NAV

rel|path|lens
up|.|-
up|src|crate `chertmem`: lib + bin `chert`, 1 node per module
self|src/install|IDE wiring: hooks & MCP registration, status, doctor
sib|src/config|`chert.toml` scopes, merge, defaults, validation
sib|src/store|`catalog.db`, partition files, writes, spool, tiers, refs
sib|src/grammar|caveman compress & expand, ported from cavemem 0.2.1
sib|src/hook|hook intake: event JSON on stdin → redact → append
sib|src/cache|in-memory recent window in `chert mcp`: load, fit, freshness
sib|src/search|BM25 + vector ranking across partitions, one scoring path
sib|src/mcp|MCP stdio server: 4 tools, hosts cache
sib|src/cli|`chert` verbs via clap, exit codes
sib|src/import|JSONL export for rekall, import incl. cavemem DB

## §C CONSTRAINTS

- edit IDE JSON via `serde_json`, preserve unrelated keys; TOML (Codex, later) via `toml_edit`, preserve formatting.
- backup file before first write; `uninstall` removes only entries chert wrote.

## §I INTERFACES

- cmd: `chert install`, `chert uninstall`, `chert status`, `chert doctor` (`.:§I`).

## §V INVARIANTS

V1: install then uninstall → IDE config byte-identical to before.
V2: install twice → same config as once (idempotent).
V3: `doctor` exits 1 naming each broken wiring, catalog drift (`src/store:V6`), stale spool.

## §T TASKS

id|status|task|cites
T1|.|Claude Code settings: hooks & MCP entry|V1,V2
T2|.|status & doctor|V3

## §B BUGS

id|date|cause|fix
