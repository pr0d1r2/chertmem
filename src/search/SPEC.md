# SPEC

## §G GOAL

targeted retrieval: prune partitions, query each in parallel, merge top-k w/ global term stats; optional vector re-rank blended by `search.alpha` (D17, D18).

## §N NAV

rel|path|lens
up|.|-
up|src|crate `chertmem`: lib + bin `chert`, 1 node per module
self|src/search|BM25 + vector ranking across partitions, one scoring path
sib|src/config|`chert.toml` scopes, merge, defaults, validation
sib|src/store|`catalog.db`, partition files, writes, spool, tiers, refs
sib|src/grammar|caveman compress & expand, ported from cavemem 0.2.1
sib|src/hook|hook intake: event JSON on stdin → redact → append
sib|src/cache|in-memory recent window in `chert mcp`: load, fit, freshness
sib|src/mcp|MCP stdio server: 4 tools, hosts cache
sib|src/cli|`chert` verbs via clap, exit codes
sib|src/install|IDE wiring: hooks & MCP registration, status, doctor
sib|src/import|JSONL export for rekall, import incl. cavemem DB
sib|src/mine|behaviour findings & awareness report

## §C CONSTRAINTS

- 1 scoring fn for cached & disk rows (D17).
- BM25 IDF from global term stats in catalog (`fts5vocab`), ⊥ per-partition IDF.
- embeddings off by default; provider `ollama` in v1; `local` later (`candle` | `fastembed`). vector scan brute-force SIMD cosine until a node measures need for ANN.
- filters: project, since, kind, session.

## §I INTERFACES

- fn (as built, wave 4): `Engine::open(cfg, project_root)`; `Engine::run(&Query, Option<&cache::Handle>) -> Vec<Hit{reference, score, snippet, session, ts}>`; `Query { text, limit, since, project, kind, session, semantic }`; `Engine::timeline(session, around: Option<&Ref>, limit) -> Vec<Entry{reference, kind, ts}>`.
- fn (T9): `search::index() -> Box<dyn cache::Index>` for `Cache::start_indexed`; `src/mcp` starts its cache w/ it.

## §R RESEARCH

id|topic|finding|src
R1|term stats, wave 4|store exposes ⊥ FTS5 or `fts5vocab` ∴ stats in search-owned `<data_dir>/search/terms.db`, ⊥ `catalog.db`: per partition last seq counted, tail past it; new `generation` or counted partition gone → recount; generation re-read after pass, moved → rollback & retry. 1 tokenizer (alphanumeric runs, lowercased) ∀ count & score. relies on `src/store` T12 (generation ⊥ repeats)|`terms.rs`, `token.rs`
R2|ranking, wave 4|BM25 k1 1.2 b 0.75, idf `ln(1+(N-df+.5)/(df+.5))`; ties ts desc, id desc. semantic: top 4×limit embedded, `(1-α)·bm25/max + α·cos`. ollama `POST /api/embed`, model `nomic-embed-text` (⊥ `chert.toml` key yet), std TCP, ⊥ TLS; openai → keyword + 1 warning (R5). real-ollama test skipped unless `CHERT_TEST_OLLAMA=1`|`rank.rs`, `blend.rs`, `ollama.rs`
R3|query grammar, wave 6|query `q.text` compressed at `compression.grammar.intensity()` before `token::terms` ∴ stats & score see stored form; snippet cuts stored body. lone word compresses ≡ inside prose, but kept spans keep long form (`middleware_x`, inline code, `src/middleware.rs`, `middleware-based`) & dropped phrases remove words (`kind` in `kind of`, `think` in `i think that`) ∴ such rows miss the lone word; kept spans fixed by R4|`search.rs`, `tests.rs`
R4|both forms, wave 7|`Asked`: terms = compressed query's ∪ plain token whose lone compression keeps ≥1 token, sorted, deduped ∴ alike form counts once, a row w/ `mw` & `middleware` scores each once; `the`, `a` stay out, lone `i`, `that` stay in. re-rank embeds compressed query (≡ form of stored bodies), raw text when it holds ⊥ term|`search.rs`, `tests.rs`
R5|no v1 client, wave 7|openai & `semantic` → `None` embedder, `Unsupported` text logged once per `Engine` via private `Warn` (`cache::Log`, stderr default); ⊥ public API change|`search.rs`, `blend/tests.rs`
R6|FTS5 candidates, wave 8|disk path: ∀ pruned partition `Store::matches(path, terms, filter)` (any term) ⊥ `tail` of every row; worker stores pooled in `Engine` across queries, dropped when `generation` moves. terms.db kept (R1): `Store::frequencies` gives rows & df, ⊥ token total ∴ ⊥ BM25 avgdl (R2); its df counts `unicode61` tokens (diacritics stripped, `café` looks up 0) while tf counts search tokens. tokenizers ≡ on ASCII (`[a-z0-9]` runs, lowercased; ∀ ASCII char tested) ∴ candidates ⊇ rows scoring > 0, V1 V2 hold. non-ASCII: unicode61 joins over chars search splits on (marks it strips like U+0301, private use, unassigned) ∴ a fragment of such a run (`e` in `e`+U+0301+`x`) found from cache, ⊥ from disk. bench, release, M1 Pro, 10^5 rows, ~10 weekly partitions, 10 of 1000 words/row, `w1 w2`, median of 21 warm runs: full scan 33.4 ms → FTS 13.9 ms → + pool 2.8 to 3.4 ms|`merge.rs`, `merge/tests.rs`, `token/tests.rs`
R7|postings, wave 10|`Postings: cache::Index`: term → `Vec<(slot u32, tf u32)>`, per slot key & dl u32; replace drops old row's postings (scan of its terms' lists, slot orphaned), `reset` frees all; > `u32` slots → `None` ∴ rows scored whole. warm path `with_index`: union postings of `Asked` terms (both forms), `Rows::get` per hit (ts filter), kind & session per hit, score from counts via same `bm25` fn as `score` ∴ bits ≡ (V1), clone only kept; other index (`Rowless`) → every row scored; `None` → store. benches `recent_warm`, `recent_cold`, `semantic_rerank` measured w/ bench cache on `start_indexed` (1-line `benches/latency/ops.rs` change, ⊥ in this node). release, M1 Pro, 2026-10-25, criterion median ms before → after, warm: 1k-p52 0.77 → 0.63, 10k-p52 1.72 → 0.63, 100k-p52 11.3 → 0.69, 1M-p52 109 → 1.81, 100k-p1 136 → 1.52, 100k-p4 147 → 1.78, 100k-p520 12.3 → 9.35 (769 rows; rest = 520-partition refresh); semantic 1.43 / 3.49 / 13.1 / 112 / 145 / 146 / 13.0 → 1.23 / 2.62 / 2.36 / 3.83 / 3.37 / 3.91 / 9.92; cold ≡ (2.17 / 1.77 / 2.39 / 8.21 / 17.7 / 7.77 / 10.4 → 1.52 / 1.58 / 2.33 / 7.49 / 17.4 / 7.99 / 10.6). live `Index::bytes` a row: 398 at 100k-p1 (10^5 rows), 414 at 1M-p52 (76923 rows), 438 at 100k-p52, 1606 at 1k (vocab ⊥ amortized) ∴ under `src/cache` 512 B estimate at ≥ 10^4 rows|`postings.rs`, `rank.rs`, `postings/tests.rs`
R8|open files, wave 13|1 WAL connection = file + `-wal` + `-shm` fds, a 2nd on the same file in-process + 2 (`-shm` shared); before: engine store (refresh) & pool each 1 conn ∀ partition ∴ 2622 fds after 1 search at 520 partitions. cap per store (`Store::keep_open`, LRU, default `Store::KEEP_OPEN` 64, src/store T20) ∴ every caller bounded, ⊥ global budget (needs a shared map of `!Sync` connections). engine: own store 4 (`merge::OWN`) + pool 60 (`BUDGET`) = 64; workers ≤ 12 ∴ share 60/12 = 5 ≥ ceil(52/12) ∴ 1 year of weekly partitions stays open across queries (T6). refresh skips a partition whose catalog checksum = last counted (`seen` table in terms.db; partition written before catalog, src/store V6) ∴ 4 conns ⊥ reopen ∀ partition per query. after: 198 fds (4 before) at 520 partitions; full suite passes under `ulimit -n 256` (`nix develop --command sh -c 'ulimit -n 256; cargo nextest run --workspace'`; nix develop resets the limit, so set it inside). bench release, M1 Pro (10 workers), 2026-10-25, criterion median ms before → after: `recent_cold` 100k-p52 2.09 → 1.49, 100k-p520 10.2 → 2.36; `cross_partition` 100k-p52 15.2 → 14.1, 100k-p520 126 → 396: 460 of 520 reopened per query, ~0.6 ms each & serialized in SQLite's unix VFS (10 threads × 52 opens 205 ms ~ 520 serial); `SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE` 388 ∴ ⊥ kept|`merge.rs`, `terms.rs`, `merge/tests.rs`, `terms/tests.rs`

## §V INVARIANTS

V1: same row → same score whether from cache or disk (`.:V1`).
V2: merged ranking ≡ ranking over 1 unpartitioned table of same rows (property test).
V3: `semantic = false` | provider off → ⊥ network (`.:V6`).
V4: ties broken by ts desc then id desc ∴ order total & deterministic.
V5: query text goes through the same grammar as stored bodies before scoring ∴ a word stored compressed (`middleware` → `mw`) is found by its plain form (B1).
V6: search & the store pool hold a bounded number of open files (≤ 64 partition connections) ∀ partition count ∴ every query works under a 256 fd limit (macOS default) (B2).

## §T TASKS

id|status|task|cites
T1|x|global term stats in catalog|V2
T2|x|parallel per-partition query & merge|V2,V4
T3|x|scoring fn shared w/ cache|V1
T4|x|ollama provider, alpha blend|V3
T5|x|compress query terms w/ `compression.grammar` before tokenizing; test plain word finds compressed body|V5
T6|x|candidate rows via store FTS5 MATCH per partition, ⊥ full scan per query ∴ `.:§C` search < 10 ms holds at 10^5 rows; benchmark it: median ~3 ms (R6)|V1,V2
T7|x|query both the plain & the compressed form of each term ∴ a word kept long inside an identifier, inline code, path or compound (`src/middleware.rs`) is found again (R3); semantic re-rank embeds the same text it scores|V5
T8|x|`embedding.provider` w/o a v1 client (openai) → keyword ranking w/ 1 log warning, ⊥ every search an error (mcp passes `semantic = true`)|V3
T9|x|warm recent-window search < 10 ms at 100k rows a bucket & 1M rows (benches `recent_warm`): in-memory postings over cached rows ⊥ scoring every row; 136 → 1.5 ms at 100k-p1, 109 → 1.8 ms at 1M, ~400 B a row (R7)|V1,V2
T10|.|cold recent-window search < 10 ms at 100k rows in 1 bucket (`recent_cold` 100k-p1, 16.0 ms in the first baseline): profile the FTS5 disk path & fix|V1,V2
T11|x|bounded open files (B2): engine store 4 + pooled stores 60 = ≤ 64 partition connections in total (LRU close per store, src/store T20), ⊥ 1 per partition per worker; refresh skips partitions whose catalog checksum stands; V6 test counts fds at 520 partitions (2622 → 198), full suite under `ulimit -n 256` (R8)|V6

## §B BUGS

id|date|cause|fix
B1|2026-10-24|wave 4 report: query tokenized raw while stored bodies are caveman-compressed ∴ plain-word query misses compressed rows|V5
B2|2026-10-25|owner push, pre-push gate on macOS (fd limit 256): `merged_ranking_equals_1_unpartitioned_table_of_the_same_rows` fails "unable to open database file"; `merge::Pool` (T6) keeps every partition connection open (up to 3 fds each) per worker ∴ `chert mcp` search fails past ~80 partitions under default limits|V6: T11, `merge::tests::a_search_over_520_partitions_holds_a_bounded_number_of_files`
