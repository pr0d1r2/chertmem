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

- fn: `search::run(Query{text, limit, since, project, semantic}) -> Vec<Hit{ref, score, snippet, session, ts}>`.
- fn: `search::timeline(session, around, limit)`.

## §R RESEARCH

id|topic|finding|src
R1|term stats, wave 4|store exposes ⊥ FTS5 or `fts5vocab` ∴ stats in search-owned `<data_dir>/search/terms.db`, ⊥ `catalog.db`: per partition last seq counted, tail past it; new `generation` or counted partition gone → recount; generation re-read after pass, moved → rollback & retry. 1 tokenizer (alphanumeric runs, lowercased) ∀ count & score. relies on `src/store` T12 (generation ⊥ repeats)|`terms.rs`, `token.rs`
R2|ranking, wave 4|BM25 k1 1.2 b 0.75, idf `ln(1+(N-df+.5)/(df+.5))`; ties ts desc, id desc. semantic: top 4×limit embedded, `(1-α)·bm25/max + α·cos`. ollama `POST /api/embed`, model `nomic-embed-text` (⊥ `chert.toml` key yet), std TCP, ⊥ TLS; openai → error in v1. real-ollama test skipped unless `CHERT_TEST_OLLAMA=1`|`rank.rs`, `blend.rs`, `ollama.rs`

## §V INVARIANTS

V1: same row → same score whether from cache or disk (`.:V1`).
V2: merged ranking ≡ ranking over 1 unpartitioned table of same rows (property test).
V3: `semantic = false` | provider off → ⊥ network (`.:V6`).
V4: ties broken by ts desc then id ∴ order total & deterministic.

## §T TASKS

id|status|task|cites
T1|x|global term stats in catalog|V2
T2|x|parallel per-partition query & merge|V2,V4
T3|x|scoring fn shared w/ cache|V1
T4|x|ollama provider, alpha blend|V3

## §B BUGS

id|date|cause|fix
