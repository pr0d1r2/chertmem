# SPEC

## §G GOAL

`chert hook run <event>`: read 1 IDE event from stdin, strip `<private>`, skip excluded paths, compress, append; injecting events (below) then print context; exit. fastest path in crate (`.:§C` < 5 ms).

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
sib|src/mcp|MCP stdio server: 4 tools, hosts cache
sib|src/cli|`chert` verbs via clap, exit codes
sib|src/install|IDE wiring: hooks & MCP registration, status, doctor
sib|src/import|JSONL export for rekall, import incl. cavemem DB
sib|src/mine|behaviour findings & awareness report

## §C CONSTRAINTS

- sync, ⊥ thread pool, ⊥ cache, ⊥ network.
- dialect per IDE; v1 = Claude Code (D24). other IDEs need recorded payload fixtures first.
- summaries (cavemem 0.2.1, `.:R1`): `stop` → turn summary from `last_assistant_message`; `session-end` → session summary = first 20 turn summaries; ⊥ model.
- injection (`.:V17`): `session-start` w/ `source = startup` (resume | compact → nothing) & `user-prompt-submit` iff `inject.on_prompt`. reading open ∴ replays spool (`src/store:V10`). stdout = only `hookSpecificOutput.additionalContext` JSON. text: last 3 other sessions' summaries as gists + refs, then top findings; cut at `inject.max_tokens` (o200k).
- unknown fields ignored; unreadable payload → spool raw, exit 0.

## §I INTERFACES

- cmd: `chert hook run session-start|user-prompt-submit|post-tool-use|stop|session-end [--ide NAME]`.
- dir: `tests/hook/fixtures/<ide>/<event>.json` recorded payloads.

## §V INVARIANTS

V1: `<private>…</private>` removed & `privacy.exclude` paths unread before compress (`.:V8`).
V2: exit 0 ∀ payload, incl. malformed; malformed → spool, ⊥ lost (`.:V2`).
V3: ∀ fixture → exactly 1 observation (or 0 when excluded), same bytes ∀ run (`.:V3`).
V4: `inject.on_prompt` false → `user-prompt-submit` prints nothing & opens store write-only.
V5: injected text ≤ `inject.max_tokens` o200k tokens ∀ fixture & store size; ⊥ full bodies (`.:V17`).
V6: `session-end` summary ≡ concat of first 20 turn summaries of that session, same bytes ∀ run.

## §T TASKS

id|status|task|cites
T1|.|record Claude Code fixtures ∀ 5 events|V3
T2|.|Claude Code dialect → event struct|V3
T3|.|redaction & exclusion|V1
T4|.|malformed → spool|V2
T5|.|measure: Claude Code resends? event id field per hook? → §R (`.:V16`)|V3
T6|.|injection: session-start default, prompt opt-in, budget cut, stdout shape|V4,V5
T7|.|turn summary on stop, session roll-up on session-end|V6

## §B BUGS

id|date|cause|fix
