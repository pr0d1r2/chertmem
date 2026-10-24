# SPEC

## §G GOAL

caveman grammar: `compress` & `expand` at intensity `lite|full|ultra` (`off` = identity, ⊥ grammar applied), byte-for-byte equal to cavemem 0.2.1 (D22). code, URLs, paths, identifiers, numbers ⊥ touched.

## §N NAV

rel|path|lens
up|.|-
up|src|crate `chertmem`: lib + bin `chert`, 1 node per module
self|src/grammar|caveman compress & expand, ported from cavemem 0.2.1
sib|src/config|`chert.toml` scopes, merge, defaults, validation
sib|src/store|`catalog.db`, partition files, writes, spool, tiers, refs
sib|src/hook|hook intake: event JSON on stdin → redact → append
sib|src/cache|in-memory recent window in `chert mcp`: load, fit, freshness
sib|src/search|BM25 + vector ranking across partitions, one scoring path
sib|src/mcp|MCP stdio server: 4 tools, hosts cache
sib|src/cli|`chert` verbs via clap, exit codes
sib|src/install|IDE wiring: hooks & MCP registration, status, doctor
sib|src/import|JSONL export for rekall, import incl. cavemem DB
sib|src/mine|behaviour findings & awareness report

## §C CONSTRAINTS

- pure fns, ⊥ I/O, ⊥ alloc-heavy regex per call (compile once).
- oracle = cavemem 0.2.1 JavaScript, tarball pinned by hash, run by Node in test only (`.:§C`). ⊥ Node in release build.
- cavemem MIT copyright notice in `THIRD-PARTY-NOTICES.md` & module header.

## §I INTERFACES

- fn: `grammar::compress(text, Intensity) -> String`; `grammar::expand(text) -> String`.
- dir: `tests/grammar/corpus/` input texts; `tests/grammar/oracle/` outputs written by cavemem.

## §V INVARIANTS

V1: ∀ corpus text & intensity: `compress` ≡ oracle bytes; `expand(compress(x))` ≡ oracle expand bytes.
V2: fenced code, inline code, URLs, paths, identifiers, version numbers survive `compress` byte for byte.
V3: deterministic: same input → same bytes (`.:V9`).

## §T TASKS

id|status|task|cites
T1|.|corpus + oracle runner, cavemem tarball pinned by hash|V1
T2|.|port `compress` per intensity|V1,V2,V3
T3|.|port `expand`|V1
T4|.|MIT notice|-

## §B BUGS

id|date|cause|fix
