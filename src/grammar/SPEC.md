# SPEC

## §G GOAL

caveman grammar: `compress` & `expand` at intensity `lite|full|ultra` (`off` = identity, ⊥ grammar applied), byte-for-byte equal to cavemem 0.2.1 (D22) except code spans, which chert ⊥ alters (V4). code, URLs, paths, identifiers, numbers ⊥ touched.

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

## §R RESEARCH

id|topic|finding|src
R1|cavemem 0.2.1 grammar 2026-10-24|last pass `/[ \t]+([.,;:!?])/g` → `$1` runs on whole output ∴ blank before mark inside fence / inline code dropped (`` `a .b` `` → `` `a.b` ``); V1 ≡ oracle keeps it, V2 holds ∀ span ⊥ such blank|`dist/chunk-T35V7EPZ.js` `compress`; `src/grammar/tests.rs`
R2|expand dead keys|`w/` `w/o` `b/c`: `/x` tokenized as path ∴ ⊥ expanded; camelCase-looking words (`tHe`) kept as identifiers|`tokenize` probe

## §V INVARIANTS

V1: ∀ corpus text & intensity: `compress` ≡ oracle bytes; `expand(compress(x))` ≡ oracle expand bytes. [superseded by V4]
V2: fenced code, inline code, URLs, paths, identifiers, version numbers survive `compress` byte for byte.
V3: deterministic: same input → same bytes (`.:V9`).
V4: ∀ corpus text & intensity: `compress` ≡ oracle bytes & `expand(compress(x))` ≡ oracle expand bytes, except 1 deliberate deviation (owner 2026-10-24, R1): final pass `[ \t]+([.,;:!?])` → `$1` skips fenced & inline code ∴ V2 holds ∀ span. golden files stay as cavemem wrote them; ∀ case the deviation changes is listed in a test w/ its expected bytes.

## §T TASKS

id|status|task|cites
T1|x|corpus + oracle runner, cavemem tarball pinned by hash|V1
T2|x|port `compress` per intensity|V1,V2,V3
T3|x|port `expand`|V1
T4|x|MIT notice|-
T5|.|punctuation pass skips fenced & inline code; tests list each golden case it changes|V2,V4

## §B BUGS

id|date|cause|fix
