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
R3|V4 deviation vs corpus 2026-10-24|0 of 24 corpus cases change: ⊥ corpus text has blank before `.,;:!?` inside code ∴ `DEVIATES` empty, golden files ≡ chert. deviation pinned by `CODE_DEVIATIONS` (cavemem bytes = chert bytes + pass once more). prose next to kept non-code spans still loses its blank (`in ./config` → `in./config`)|`src/grammar/tests.rs`
R4|inline code after a fence|cavemem tokenizer takes `` ` c ` `` from the fence's last tick as inline code, it loses to the fence ∴ a following `` `d ,` `` is prose & loses its blank; fixed by V5 (b) (T6)|`CODE_DEVIATIONS`
R5|V5 (b), (c) vs corpus 2026-10-24|(b) changes 0 cases; (c) changes `urls_paths.{lite,full,ultra}.{compressed,expanded}` (`in ./config`, `is ../shared`), in `DEVIATES`. (c) = blank before a path kept; ⊥ other kept span starts w/ `.,;:!?` ∴ ≡ "before any protected token"|`src/grammar/tests.rs`
R6|(d) rule & corpus 2026-10-24|path span = article word at `\b`, any case, + `/`&path byte or `.`&alnum, then `[A-Za-z0-9._\-/]*`; `a.`+blank, `a/`+blank ≡ cavemem; `a.The` kept. 0 corpus cases change|`ARTICLE_DEVIATIONS`

## §V INVARIANTS

V1: ∀ corpus text & intensity: `compress` ≡ oracle bytes; `expand(compress(x))` ≡ oracle expand bytes. [superseded by V5]
V2: fenced code, inline code, URLs, paths, identifiers, version numbers survive `compress` byte for byte.
V3: deterministic: same input → same bytes (`.:V9`).
V4: ∀ corpus text & intensity: `compress` ≡ oracle bytes & `expand(compress(x))` ≡ oracle expand bytes, except 1 deliberate deviation (owner 2026-10-24, R1): final pass `[ \t]+([.,;:!?])` → `$1` skips fenced & inline code ∴ V2 holds ∀ span. golden files stay as cavemem wrote them; ∀ case the deviation changes is listed in a test w/ its expected bytes. [superseded by V5]
V5: as V4, w/ 3 deliberate deviations from cavemem 0.2.1 (owner 2026-10-24), each listed in a test w/ its expected bytes: (a) final punctuation pass skips fenced & inline code (R1); (b) inline code right after a fence = code too, ⊥ swallowed by the fence's last tick (R4); (c) blank before a path kept: `in ./config/chert.toml` ⊥ → `in./config/chert.toml`. golden files stay as cavemem wrote them.

## §T TASKS

id|status|task|cites
T1|x|corpus + oracle runner, cavemem tarball pinned by hash|V1
T2|x|port `compress` per intensity|V1,V2,V3
T3|x|port `expand`|V1
T4|x|MIT notice|-
T5|x|punctuation pass skips fenced & inline code; tests list each golden case it changes|V2,V4
T6|x|deviations (b) & (c) of V5, RED → GREEN; tests list each golden case they change|V2,V5
T7|x|deviation (d) of V2: a path or filename whose first segment is an article word (`a/b.ipynb`, `a.txt`, `the.txt`) stays whole; ⊥ `/b.ipynb`, `cat.txt`. cavemem 0.2.1 drops it (B1)|V2,V5

## §B BUGS

id|date|cause|fix
B1|2026-10-24|mine worker, confirmed on cavemem 0.2.1 oracle: at full & ultra the article filter splits a leading `a` or `the` off a path or filename (`cat a.txt` → `cat.txt`, `a/b.ipynb` → `/b.ipynb`) ∴ paths ⊥ survive (V2), mined hot files mangled|V2
