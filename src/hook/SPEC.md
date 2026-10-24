# SPEC

## §G GOAL

`chert hook run <event>`: read 1 IDE event from stdin, strip `<private>`, skip excluded paths, compress, append, exit. fastest path in crate (`.:§C` < 5 ms).

## §N NAV

rel|path|lens
up|.|-
up|src|crate `chertmem`: lib + bin `chert`, 1 node per module
self|src/hook|hook intake: event JSON on stdin → redact → append
sib|src/config|`chert.toml` scopes, merge, defaults, validation
sib|src/store|`catalog.db`, partition files, writes, spool, tiers, refs
sib|src/grammar|caveman compress & expand, ported from cavemem 0.2.1
sib|src/cache|in-memory recent window in `chert mcp`: load, fit, freshness
sib|src/search|BM25 + vector ranking across partitions, one scoring path

## §C CONSTRAINTS

- sync, ⊥ thread pool, ⊥ cache, ⊥ network.
- dialect per IDE; v1 = Claude Code (D24). other IDEs need recorded payload fixtures first.
- unknown fields ignored; unreadable payload → spooled raw + exit 0 ∴ IDE ⊥ blocked.

## §I INTERFACES

- cmd: `chert hook run session-start|user-prompt-submit|post-tool-use|stop|session-end [--ide NAME]`.
- dir: `tests/hook/fixtures/<ide>/<event>.json` recorded payloads.

## §V INVARIANTS

V1: `<private>…</private>` removed & `privacy.exclude` paths unread before compress (`.:V8`).
V2: exit 0 ∀ payload, incl. malformed; malformed → spool, ⊥ lost (`.:V2`).
V3: ∀ fixture → exactly 1 observation (or 0 when excluded), same bytes ∀ run (`.:V3`).

## §T TASKS

id|status|task|cites
T1|.|record Claude Code fixtures ∀ 5 events|V3
T2|.|Claude Code dialect → event struct|V3
T3|.|redaction & exclusion|V1
T4|.|malformed → spool|V2

## §B BUGS

id|date|cause|fix
