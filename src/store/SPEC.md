# SPEC

## §G GOAL

embedded hypertable: `catalog.db` + 1 SQLite file per partition (`by` dims × time bucket × shard). writes, reads by id, partition pruning, hot → warm → cold tiers, spool, ref resolution (D12, D13, D28).

## §N NAV

rel|path|lens
up|.|-
up|src|crate `chertmem`: lib + bin `chert`, 1 node per module
self|src/store|`catalog.db`, partition files, writes, spool, tiers, refs
sib|src/config|`chert.toml` scopes, merge, defaults, validation
sib|src/grammar|caveman compress & expand, ported from cavemem 0.2.1
sib|src/hook|hook intake: event JSON on stdin → redact → append
sib|src/cache|in-memory recent window in `chert mcp`: load, fit, freshness
sib|src/search|BM25 + vector ranking across partitions, one scoring path
sib|src/mcp|MCP stdio server: 4 tools, hosts cache
sib|src/cli|`chert` verbs via clap, exit codes
sib|src/install|IDE wiring: hooks & MCP registration, status, doctor

## §C CONSTRAINTS

- `rusqlite` `bundled`; ∀ file WAL; busy timeout then spool, ⊥ block hook > `.:§C` target.
- path: `chunks/<project>/<bucket>[.<shard>].db[.zst]`; shard = hash(project, id) mod `shards`.
- catalog row ∀ partition: dims, `[t0,t1)`, rows, bytes, state ∈ {hot, warm, cold}, checksum, spec it was written under.
- observation: ULID id, `seq INTEGER PRIMARY KEY AUTOINCREMENT`, idempotency key unique, ts, session, kind, body (compressed), FTS5 external-content index.
- 1 connection per partition, ⊥ `ATTACH` (`.:R5`).
- `generation` counter in catalog, bumped on any delete | redaction | rewrite (D15).

## §I INTERFACES

- fn: `Store::open(cfg)`; `append(event) -> Ref`; `get(&[Ref]) -> Vec<Found|Dropped>`; `partitions(filter) -> Vec<Partition>`; `seal`, `compact`, `drop`, `repartition`.
- fn: `Ref::parse("chert:<prefix>")`; `Ref::short(id, project)` → shortest unique prefix ≥ 8.
- dir: `spool/` append-only JSONL, replayed at next `open`.

## §V INVARIANTS

V1: obs stored in bucket of own ts, ⊥ session start ts (D13).
V2: same idempotency key twice → 1 row, 2nd append returns 1st ref (`.:V3`).
V3: lock timeout → event in spool & `append` returns Ok; next `open` replays spool before any read (`.:V2`).
V4: read path picks partitions from catalog bounds only (`.:V4`); test: change config, old rows still found.
V5: ref resolves ∀ tier & after `repartition`; dropped partition → `Dropped` (`.:V5`).
V6: catalog write after partition write; `doctor` rebuilds catalog from files & equals pre-crash catalog.
V7: warm & cold partitions read-only; write into them ⊥.

## §T TASKS

id|status|task|cites
T1|.|schema: catalog & partition tables, FTS5 external content|V1,V7
T2|.|append w/ idempotency key & ULID|V1,V2
T3|.|spool & replay|V3
T4|.|pruning by catalog bounds|V4
T5|.|refs: parse, shortest prefix, resolve, dropped|V5
T6|.|seal → warm, compact → cold (`VACUUM INTO` + zstd dict), drop, repartition|V5,V7
T7|.|catalog rebuild for `doctor`|V6

## §B BUGS

id|date|cause|fix
