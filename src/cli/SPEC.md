# SPEC

## §G GOAL

clap front end: parse ∀ `.:§I` verb, call owning module, map result to exit 0 | 1 | 2.

## §N NAV

rel|path|lens
up|.|-
up|src|crate `chertmem`: lib + bin `chert`, 1 node per module
self|src/cli|`chert` verbs via clap, exit codes
sib|src/config|`chert.toml` scopes, merge, defaults, validation
sib|src/store|`catalog.db`, partition files, writes, spool, tiers, refs
sib|src/grammar|caveman compress & expand, ported from cavemem 0.2.1
sib|src/hook|hook intake: event JSON on stdin → redact → append
sib|src/cache|in-memory recent window in `chert mcp`: load, fit, freshness
sib|src/search|BM25 + vector ranking across partitions, one scoring path
sib|src/mcp|MCP stdio server: 4 tools, hosts cache
sib|src/install|IDE wiring: hooks & MCP registration, status, doctor
sib|src/import|JSONL export for rekall, import incl. cavemem DB

## §C CONSTRAINTS

- clap derive (`paradigm/V29`); `completions` & `man` generated from same definition.
- ⊥ logic beyond dispatch & output formatting.

## §I INTERFACES

- fn: `cli::run(args) -> ExitCode`.

## §V INVARIANTS

V1: help text, completions & man page generated from 1 clap definition ∴ ⊥ drift.
V2: exit 0 ok · 1 finding · 2 usage | unreadable input (`.:§I`); test per verb.

## §T TASKS

id|status|task|cites
T1|.|clap definition ∀ verbs|V1
T2|.|exit-code tests|V2

## §B BUGS

id|date|cause|fix
