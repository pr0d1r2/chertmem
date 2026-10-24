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

## §R RESEARCH

id|topic|finding|src
R1|rekall candidate, wave 4|candidate = `{id, src, text, at}`; `catch` reads transcript roots, dialect per agent; ⊥ `chert` dialect agreed (`.:T14` ⊥ filed) ∴ `--for rekall` shape unsettled, T3 `.`; plain export only|`.:R3`
R2|cavemem 0.2.1 mapping, wave 4|obs `session_id` `kind` `ts` + session `ide` → event as is; `content` → `<private>` stripped (hook rules; cavemem `redactSecrets` may be off) then compressed at `compression.grammar` iff `compressed = 0`; identity `cavemem:<session_id>:<id>` ∴ re-import → 1st ref (`src/store:V9`). summaries: `session_id` `scope` `ts` + session `ide`, body as obs; ⊥ identity in store ∴ matched to held summaries equal in every field, each held answers for 1. dropped: `intensity`, obs `metadata`, session `cwd` `started_at` `ended_at` `metadata`, FTS, `embeddings`. open `file:<path>?mode=ro`, `schema_version` 2 required. all rows → store's project, ⊥ split by session `cwd`|cavemem `dist/chunk-T35V7EPZ.js` `SCHEMA_SQL`, `Storage` inserts
R3|export line, wave 4|1 JSON object per line, keys sorted: `body` `id` `ide` `identity` (null if ⊥) `kind` `project` `ref` (`chert:<ref>`) `session` `ts`; body = stored bytes ∴ ⊥ redacted text (`.:V8`). import of it blocked: store issues new id & ref per append ∴ V1 needs store fn keeping given id & ref (T1 `~`)|`src/import/export.rs`

## §V INVARIANTS

V1: export → import into empty store → same observations, same refs.
V2: cavemem import idempotent: twice → same rows (`.:V3`).
V3: cavemem DB file unchanged after import (checksum before ≡ after).

## §T TASKS

id|status|task|cites
T1|~|JSONL export & import; export done, import waits on store append keeping id & ref (R3)|V1
T2|x|cavemem 0.2.1 import from fixture DB|V2,V3
T3|.|`--for rekall` shape, agreed via rekall issue (`.:T14`, R1)|-
T4|x|import cavemem `summaries` (turn & session scope) w/ their sessions|V1,V2
T5|.|cavemem rows split by session `cwd`: each session → chert project of its own cwd (same id the hook derives); missing cwd → current project, counted in report (owner 2026-10-24)|V2

## §B BUGS

id|date|cause|fix
