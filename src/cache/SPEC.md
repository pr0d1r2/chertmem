# SPEC

## §G GOAL

hold recent `cache.window` of current project in memory inside `chert mcp` (D14): rows in the store's `(ts, id)` order (R1), keyword index, embedding matrix. bounded by `cache.max_bytes`; never stale (D15, D16).

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
sib|src/import|JSONL export for rekall, import incl. cavemem DB
sib|src/mine|behaviour findings & awareness report

## §C CONSTRAINTS

- warm in background thread; until warm, queries go to `store` (D14).
- freshness: `PRAGMA data_version` before ∀ query; tail by `seq > last_seq`; catalog `generation` change → full reload (D15).
- fit: estimate from catalog stats before load; `warn` keeps newest days that fit & says how many, `shrink` same after 1 warning, `fail` → disk only (D16).
- memory check: warn when `max_bytes` > ~25% available memory.

## §I INTERFACES

- fn: `Cache::start(store, cfg) -> Handle`; `Handle::query(q) -> Rows`; `Handle::plan() -> Plan{estimate, limit, days_kept}`; `Cache::plan(&store, cfg) -> Plan` (system clock), `Cache::plan_at(&store, cfg, now) -> Plan`: same estimate as `Handle::plan` (1 `fit_at`), ⊥ thread, ⊥ row load, ⊥ log.
- cmd: `chert cache plan` prints `Cache::plan_at`'s `Plan`; exit 1 when estimate > limit.

## §R RESEARCH

id|topic|finding|src
R1|fit, wave 3|row = body + 640 B: `Observation` 200 inline, `(ts, id)` key 32, 8 heap strings ~200, B-tree slack, rounded up; bucket share ∝ its elapsed part; mo = 31d; `warn` logs ∀ load, `shrink` once|`fit.rs`
R2|memory, lead|Linux `MemAvailable`; macOS total `hw.memsize` (⊥ cheap available figure); else ⊥ check; `Env.memory` injected|`memory.rs`
R3|query, lead|`q` = `store::Filter` → rows in store `(ts, id)` order (§G "by id" → reconcile); memory answers only same project & `since` ≥ window start|`window.rs`
R4|tests, lead|V1: test binary re-invoked as 2nd writer process. V3: ⊥ store `drop` on base ∴ raw SQL edit + `generation` bump|`window/tests.rs`
R5|later, lead|keyword index & embedding matrix → `src/search`; cache holds rows only. `chert cache plan` verb → `src/cli`|-

## §V INVARIANTS

V1: ∀ query: result via cache ≡ result via `store` alone (`.:V1`); property test w/ concurrent writer process.
V2: commit by other process visible to next query (data_version + seq tail); ⊥ ULID order used for tail.
V3: delete | redaction | rewrite elsewhere → next query sees it (generation reload).
V4: estimate > `max_bytes` → warning names estimate & limit (`.:V11`); ⊥ text into MCP tool results.

## §T TASKS

id|status|task|cites
T1|x|load window from partitions, background warm|V1
T2|x|data_version + seq tail|V2
T3|x|generation reload|V3
T4|x|estimate, overflow policies, memory check, `cache plan`|V4
T5|x|`Cache::plan(store, cfg) -> Plan` ⊥ warm-up thread; `chert cache plan` uses it (src/cli R4)|V4

## §B BUGS

id|date|cause|fix
