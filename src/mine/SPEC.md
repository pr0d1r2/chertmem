# SPEC

## §G GOAL

`chert mine` & `chert report`: behaviour findings w/ evidence refs — repeated failing commands & errors, repeated command sequences, hot files, token sinks (D23, D27). user corrections left to rekall.

## §N NAV

rel|path|lens
up|.|-
up|src|crate `chertmem`: lib + bin `chert`, 1 node per module
self|src/mine|behaviour findings & awareness report
sib|src/config|`chert.toml` scopes, merge, defaults, validation
sib|src/store|`catalog.db`, partition files, writes, spool, tiers, refs
sib|src/grammar|caveman compress & expand, ported from cavemem 0.2.1
sib|src/hook|hook intake: event JSON on stdin → redact → append
sib|src/cache|in-memory recent window in `chert mcp`: load, fit, freshness
sib|src/search|BM25 + vector ranking across partitions, one scoring path
sib|src/mcp|MCP stdio server: 4 tools, hosts cache
sib|src/cli|`chert` verbs via clap, exit codes
sib|src/install|IDE wiring: hooks & MCP registration, status, doctor
sib|src/import|JSONL export for rekall, import incl. cavemem DB

## §C CONSTRAINTS

- mine each sealed partition once; findings stored per partition; hot partition mined on demand.
- threshold `mining.min_occurrences`.
- ⊥ change anything outside chert's data dir (`.:§C`).

## §I INTERFACES

- cmd: `chert mine [--since T] [--format text|json]`; `chert report [--format text|json|md]`.
- finding: `{id, kind, count, refs[], summary}`; `id` = hash of pattern.

## §V INVARIANTS

V1: same partitions & version → same findings, same ids, same order (`.:V9`).
V2: ∀ finding carries ≥ `min_occurrences` refs, each resolvable (`.:V5`).
V3: findings ⊥ written under `.rekall/` (`.:V7`).

## §T TASKS

id|status|task|cites
T1|.|failing-command & error signal|V1,V2
T2|.|command-sequence signal|V1,V2
T3|.|hot files & token sinks, `report`|V1
T4|.|per-partition finding store|V1

## §B BUGS

id|date|cause|fix
