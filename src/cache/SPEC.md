# SPEC

## §G GOAL

hold recent `cache.window` of current project in memory inside `chert mcp` (D14): rows sorted by id, keyword index, embedding matrix. bounded by `cache.max_bytes`; never stale (D15, D16).

## §N NAV

rel|path|lens
up|.|-
up|src|crate `chertmem`: lib + bin `chert`, 1 node per module
self|src/cache|in-memory recent window in `chert mcp`: load, fit, freshness
sib|src/config|`chert.toml` scopes, merge, defaults, validation
sib|src/store|`catalog.db`, partition files, writes, spool, tiers, refs
sib|src/grammar|caveman compress & expand, ported from cavemem 0.2.1
sib|src/hook|hook intake: event JSON on stdin → redact → append
sib|src/search|BM25 + vector ranking across partitions, one scoring path
sib|src/mcp|MCP stdio server: 4 tools, hosts cache
sib|src/cli|`chert` verbs via clap, exit codes
sib|src/install|IDE wiring: hooks & MCP registration, status, doctor

## §C CONSTRAINTS

- warm in background thread; until warm, queries go to `store` (D14).
- freshness: `PRAGMA data_version` before ∀ query; tail by `seq > last_seq`; catalog `generation` change → full reload (D15).
- fit: estimate from catalog stats before load; `warn` keeps newest days that fit & says how many, `shrink` same after 1 warning, `fail` → disk only (D16).
- memory check: warn when `max_bytes` > ~25% available memory.

## §I INTERFACES

- fn: `Cache::start(store, cfg) -> Handle`; `Handle::query(q) -> Rows`; `Handle::plan() -> Plan{estimate, limit, days_kept}`.
- cmd: `chert cache plan` prints `Plan`.

## §V INVARIANTS

V1: ∀ query: result via cache ≡ result via `store` alone (`.:V1`); property test w/ concurrent writer process.
V2: commit by other process visible to next query (data_version + seq tail); ⊥ ULID order used for tail.
V3: delete | redaction | rewrite elsewhere → next query sees it (generation reload).
V4: estimate > `max_bytes` → warning names estimate & limit (`.:V11`); ⊥ text into MCP tool results.

## §T TASKS

id|status|task|cites
T1|.|load window from partitions, background warm|V1
T2|.|data_version + seq tail|V2
T3|.|generation reload|V3
T4|.|estimate, overflow policies, memory check, `cache plan`|V4

## §B BUGS

id|date|cause|fix
