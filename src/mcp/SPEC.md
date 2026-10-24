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

## §C CONSTRAINTS

- transport stdio JSON-RPC. lib choice (`rmcp` vs hand-rolled) decided by T1 w/ §R row: binary size, deps, sync fit.
- results compressed unless `expand: true`; `id` field carries ref (`.:§I`).
- logs → stderr only; ⊥ stdout noise.

## §I INTERFACES

- mcp: `search`, `timeline`, `get_observations`, `list_sessions` per `.:§I`.

## §V INVARIANTS

V1: tool names & field names ≡ cavemem 0.2.1 (`.:R1`); golden JSON test per tool.
V2: stdout carries protocol frames only.
V3: cache warnings ⊥ in tool results (`.:V11`).

## §T TASKS

id|status|task|cites
T1|.|transport lib choice, §R row|V2
T2|.|4 tools w/ golden JSON|V1
T3|.|start cache on launch|V3

## §B BUGS

id|date|cause|fix
