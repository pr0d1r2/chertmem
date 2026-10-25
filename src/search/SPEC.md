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

## §R RESEARCH

id|topic|finding|src
R1|term stats, wave 4|store exposes ⊥ FTS5 or `fts5vocab` ∴ stats in search-owned `<data_dir>/search/terms.db`, ⊥ `catalog.db`: per partition last seq counted, tail past it; new `generation` or counted partition gone → recount; generation re-read after pass, moved → rollback & retry. 1 tokenizer (alphanumeric runs, lowercased) ∀ count & score. relies on `src/store` T12 (generation ⊥ repeats)|`terms.rs`, `token.rs`
R2|ranking, wave 4|BM25 k1 1.2 b 0.75, idf `ln(1+(N-df+.5)/(df+.5))`; ties ts desc, id desc. semantic: top 4×limit embedded, `(1-α)·bm25/max + α·cos`. ollama `POST /api/embed`, model `nomic-embed-text` (⊥ `chert.toml` key yet), std TCP, ⊥ TLS; openai → keyword + 1 warning (R5). real-ollama test skipped unless `CHERT_TEST_OLLAMA=1`|`rank.rs`, `blend.rs`, `ollama.rs`
R3|query grammar, wave 6|query `q.text` compressed at `compression.grammar.intensity()` before `token::terms` ∴ stats & score see stored form; snippet cuts stored body. lone word compresses ≡ inside prose, but kept spans keep long form (`middleware_x`, inline code, `src/middleware.rs`, `middleware-based`) & dropped phrases remove words (`kind` in `kind of`, `think` in `i think that`) ∴ such rows miss the lone word; kept spans fixed by R4|`search.rs`, `tests.rs`
R4|both forms, wave 7|`Asked`: terms = compressed query's ∪ plain token whose lone compression keeps ≥1 token, sorted, deduped ∴ alike form counts once, a row w/ `mw` & `middleware` scores each once; `the`, `a` stay out, lone `i`, `that` stay in. re-rank embeds compressed query (≡ form of stored bodies), raw text when it holds ⊥ term|`search.rs`, `tests.rs`
R5|no v1 client, wave 7|openai & `semantic` → `None` embedder, `Unsupported` text logged once per `Engine` via private `Warn` (`cache::Log`, stderr default); ⊥ public API change|`search.rs`, `blend/tests.rs`
R6|FTS5 candidates, wave 8|disk path: ∀ pruned partition `Store::matches(path, terms, filter)` (any term) ⊥ `tail` of every row; worker stores pooled in `Engine` across queries, dropped when `generation` moves. terms.db kept (R1): `Store::frequencies` gives rows & df, ⊥ token total ∴ ⊥ BM25 avgdl (R2); its df counts `unicode61` tokens (diacritics stripped, `café` looks up 0) while tf counts search tokens. tokenizers ≡ on ASCII (`[a-z0-9]` runs, lowercased; ∀ ASCII char tested) ∴ candidates ⊇ rows scoring > 0, V1 V2 hold. non-ASCII: unicode61 joins over chars search splits on (marks it strips like U+0301, private use, unassigned) ∴ a fragment of such a run (`e` in `e`+U+0301+`x`) found from cache, ⊥ from disk. bench, release, M1 Pro, 10^5 rows, ~10 weekly partitions, 10 of 1000 words/row, `w1 w2`, median of 21 warm runs: full scan 33.4 ms → FTS 13.9 ms → + pool 2.8 to 3.4 ms|`merge.rs`, `merge/tests.rs`, `token/tests.rs`

## §V INVARIANTS

V1: same row → same score whether from cache or disk (`.:V1`).
V2: merged ranking ≡ ranking over 1 unpartitioned table of same rows (property test).
V3: `semantic = false` | provider off → ⊥ network (`.:V6`).
V4: ties broken by ts desc then id desc ∴ order total & deterministic.
V5: query text goes through the same grammar as stored bodies before scoring ∴ a word stored compressed (`middleware` → `mw`) is found by its plain form (B1).

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
T9|.|warm recent-window search < 10 ms at 100k rows a bucket & 1M rows (benches `recent_warm`): in-memory postings over cached rows ⊥ scoring every row; today 140 ms at 100k-p1, 115 ms at 1M; V1 holds after `src/cache:T6`|V1,V2

## §B BUGS

id|date|cause|fix
B1|2026-10-24|wave 4 report: query tokenized raw while stored bodies are caveman-compressed ∴ plain-word query misses compressed rows|V5
