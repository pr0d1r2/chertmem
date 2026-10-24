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
sib|src/import|JSONL export for rekall, import incl. cavemem DB
sib|src/mine|behaviour findings & awareness report

## §C CONSTRAINTS

- `rusqlite` `bundled`; ∀ file WAL; busy timeout then spool, ⊥ block hook > `.:§C` target.
- path: `chunks/<project>/<bucket>[.<shard>].db[.zst]`; shard = hash(project, id) mod `shards`.
- catalog row ∀ partition: dims, `[t0,t1)`, rows, bytes, state ∈ {hot, warm, cold}, checksum, spec it was written under.
- observation: ULID id, `seq INTEGER PRIMARY KEY AUTOINCREMENT`, event identity key unique when dialect supplies one (`.:V16`), ts, session, kind, body (compressed), FTS5 external-content index.
- 1 connection per partition, ⊥ `ATTACH` (`.:R5`).
- summaries: table per partition, scope ∈ {`turn`, `session`}, keyed by session; stored compressed like bodies; read by injection (`src/hook`).
- `generation` counter in catalog, bumped on any delete | redaction | rewrite (D15).

## §I INTERFACES

- fn (as built, wave 2): `Store::open(cfg, project_root, Mode::{Write,Read})`; `append(&Event) -> Appended::{Stored(Ref),Spooled}`; `append_summary`; `get(&[Ref]) -> Vec<Lookup::{Found,Dropped}>`; `partitions`, `observations(&Filter)`; `summaries`; `replay`; `generation`. Write mode: ∀ read → `Error::WriteOnly`. wave 3: `seal`, `compact`, `drop`, `repartition`, `rebuild`, `tail`, `data_version`, `catalog_version`.
- fn: `Ref::parse("chert:<ref>")`; ref issued in `append`: shortest ULID prefix ≥ 8 ⊥ equal to issued ref in project, stored in unique column `ref`; `get` matches stored ref exactly (`.:V15`).
- dir: `spool/` append-only JSONL, replayed by next reading `open` (V10).

## §R RESEARCH

id|topic|finding|src
R1|layout, wave 2|project id = dir name + FNV-1a 64 of canonical path; dir = non-time dims, `by` order, `+`-joined; bucket `<t0>-<span>`, h & d from epoch, mo from 0000-01; shard FNV-1a(project, id); checksum FNV-1a 64 ∀ (id, body), insert order; bytes = Σ observation bodies|`layout.rs`, `calendar.rs`
R2|spool, wave 2|busy 2 ms write-only, 1 s reading; 1 file per item, renamed into place; spooled append → `Spooled`, ref issued at replay; unreadable → `spool/unreadable/`|`spool.rs`
R3|tiers, wave 3|file keeps own catalog row; cold `<path>.zst` + `.dict`, read via `thawed/<path>`; sealed spool item → `spool/sealed/`|`cold.rs`
R4|generation, wave 4|bump = max(stored + 1, now ms, mark + 1); mark = file `generation` beside `catalog.db`, rewritten per bump; new catalog seeded so|`generation.rs`
R5|tombstones, wave 4|`<path>.dropped`: JSON `{partition: [12 catalog cols], refs: [[project, ref, id, identity]]}`, before files go; rebuild restores before files; bad 1 → error. `spool_raw` → `spool/unreadable/<ULID>.hook`, as `src/hook` writes|`tombstone.rs`, `spool.rs`

## §V INVARIANTS

V1: obs stored in bucket of own ts, ⊥ session start ts (D13).
V2: same idempotency key twice → 1 row, 2nd append returns 1st ref (`.:V3`). [superseded by V9]
V3: lock timeout → event in spool & `append` returns Ok; next `open` replays spool before any read (`.:V2`). [superseded by V10]
V4: read path picks partitions from catalog bounds only (`.:V4`); test: change config, old rows still found.
V5: ref resolves ∀ tier & after `repartition`; dropped partition → `Dropped` (`.:V5`).
V6: catalog write after partition write; `doctor` rebuilds catalog from files & equals pre-crash catalog.
V7: warm & cold partitions read-only; write into them ⊥.
V8: ∀ issued ref stays exact-match unique: insert 10⁵ rows in 1 s → ∀ earlier ref still resolves to own row (`.:V15`, `.:B1`).
V9: same event identity twice → 1 row, 2nd append returns 1st ref; event w/o identity → new row every time (`.:V16`).
V10: lock timeout → event in spool & `append` returns Ok. write-only open (hook) ⊥ replays spool ∴ hook latency ⊥ grows w/ spool; ∀ reading open replays whole spool before first read ∴ reads complete (`.:V2`, `.:V1`, `.:B3`).
V11: `generation` never repeats a value, across `rebuild` too ∴ a cache holding an old value ⊥ reads stale as fresh (B1).
V12: a dropped ref stays dropped across catalog loss: drop writes a tombstone outside `catalog.db` that `rebuild` reads ∴ ⊥ reissued (B2).

## §T TASKS

id|status|task|cites
T1|x|schema: catalog & partition tables, FTS5 external content|V1,V7
T2|x|append w/ event identity key & ULID|V1,V9
T3|x|spool & replay on reading open|V10
T4|x|pruning by catalog bounds|V4
T5|x|refs: parse, shortest prefix, resolve, dropped|V5
T6|x|seal → warm, compact → cold (`VACUUM INTO` + zstd dict), drop, repartition|V5,V7
T7|x|catalog rebuild for `doctor`|V6
T8|x|ref column & exact-match resolution, cross-project lookup|V5,V8
T9|x|summaries table, turn & session scopes|V1
T10|x|w/ T6: spooled item whose partition got sealed ⊥ blocks replay forever|V7,V10
T11|x|for `src/cache`: `tail` by seq, `data_version`, `catalog_version`|`src/cache:V2`
T12|x|`generation` from a monotonic source (e.g. max(stored, now ms)), survives `rebuild`|V11
T13|x|drop tombstones under `chunks/<project>/`, read by `rebuild`|V12
T14|x|`Store::spool_raw(bytes)` for unparsable hook payloads (owner 2026-10-24); `src/hook` calls it|V10

## §B BUGS

id|date|cause|fix
B1|2026-10-24|wave 3 report: `rebuild` after a deleted catalog restarts `generation` at 1 ∴ a running cache that saw 1 earlier takes changed data as fresh, breaking `.:V1`|V11
B2|2026-10-24|wave 3 report: deleted catalog loses dropped markers ∴ a dropped ref's text could be issued again for a new row in the same ms window, breaking `.:V15`|V12
