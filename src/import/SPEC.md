# SPEC

## §G GOAL

`chert export` JSONL (1 observation per line, `ref` field; `--for rekall` shape read by rekall `catch`, D27) & `chert import` (JSONL | cavemem 0.2.1 SQLite, D21).

## §N NAV

rel|path|lens
up|.|-
up|src|crate `chertmem`: lib + bin `chert`, 1 node per module
self|src/import|JSONL export for rekall, import incl. cavemem DB
sib|src/config|`chert.toml` scopes, merge, defaults, validation
sib|src/store|`catalog.db`, partition files, writes, spool, tiers, refs
sib|src/grammar|caveman compress & expand, ported from cavemem 0.2.1
sib|src/hook|hook intake: event JSON on stdin → redact → append
sib|src/cache|in-memory recent window in `chert mcp`: load, fit, freshness
sib|src/search|BM25 + vector ranking across partitions, one scoring path
sib|src/mcp|MCP stdio server: 4 tools, hosts cache
sib|src/cli|`chert` verbs via clap, exit codes
sib|src/install|IDE wiring: hooks & MCP registration, status, doctor
sib|src/mine|behaviour findings & awareness report

## §C CONSTRAINTS

- read cavemem DB read-only; ⊥ write cavemem's files (`.:§C`).
- export ⊥ includes redacted text (`.:V8`).
- ⊥ write `.rekall/` (`.:V7`).

## §I INTERFACES

- cmd: `chert export <out.jsonl> [--for rekall]`; `chert import <in> [--from cavemem]`.

## §V INVARIANTS

V1: export → import into empty store → same observations, same refs.
V2: cavemem import idempotent: twice → same rows (`.:V3`).
V3: cavemem DB file unchanged after import (checksum before ≡ after).

## §T TASKS

id|status|task|cites
T1|.|JSONL export & import|V1
T2|.|cavemem 0.2.1 import from fixture DB|V2,V3
T3|.|`--for rekall` shape, agreed via rekall issue (`.:T14`)|-

## §B BUGS

id|date|cause|fix
