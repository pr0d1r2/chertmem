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
sib|src/mine|behaviour findings & awareness report

## §C CONSTRAINTS

- edit IDE JSON via `serde_json`, preserve unrelated keys; TOML (Codex, later) via `toml_edit`, preserve formatting.
- backup file before first write; `uninstall` removes only entries chert wrote.

## §I INTERFACES

- cmd: `chert install`, `chert uninstall`, `chert status`, `chert doctor` (`.:§I`).

## §R RESEARCH

id|topic|finding|src
R1|bytes, wave 7|⊥ `preserve_order` (`src/hook` V3 needs sorted keys) ∴ write = sorted, 2-space. 1st write → `<file>.chert.bak`; uninstall: rest ≡ backup JSON → backup bytes back; no backup & rest `{}` → file removed; else rest written, backup kept|`install.rs`
R2|wiring, wave 7|as cavemem: 1 group/event `<chert> hook run <event> --ide claude-code`, `mcpServers.chert = {command, args: [mcp]}`; appended ⊥ replacing. doctor: rebuild & diff catalog; files left in `spool/`, `sealed/`, `unreadable/` = stale|cavemem `dist/index.js` installers; `doctor.rs`

## §V INVARIANTS

V1: install then uninstall → IDE config byte-identical to before.
V2: install twice → same config as once (idempotent).
V3: `doctor` exits 1 naming each broken wiring, catalog drift (`src/store:V6`), stale spool.
V4: chert's MCP server is registered where Claude Code reads MCP servers, so `chert mcp` actually launches (B1).

## §T TASKS

id|status|task|cites
T1|x|Claude Code settings: hooks & MCP entry|V1,V2
T2|x|status & doctor|V3
T3|.|MCP entry goes to `~/.claude.json` `mcpServers.chert` (user scope), ⊥ `settings.json`, under V1 & V2 (B1)|V1,V2,V4

## §B BUGS

id|date|cause|fix
B1|2026-10-25|install wrote `mcpServers.chert` into `settings.json` as cavemem 0.2.1 does, but Claude Code's settings schema has no `mcpServers` key (MCP servers live in `~/.claude.json` or `.mcp.json`) ∴ the server is never launched|V4
