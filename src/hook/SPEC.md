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
- injection (`.:V17`): `session-start` w/ `source = startup` (resume | compact → nothing) & `user-prompt-submit` iff `inject.on_prompt`. reading open ∴ replays spool (`src/store:V10`). stdout = only `hookSpecificOutput.additionalContext` JSON. text: last 3 other sessions' summaries as gists + refs, then top findings; cut at `inject.max_tokens` counted tokens: structured estimate ≥ o200k (R7); exact o200k with feature `bpe`.
- unknown fields ignored; unreadable payload → spool raw, exit 0.

## §I INTERFACES

- cmd: `chert hook run session-start|user-prompt-submit|post-tool-use|post-tool-use-failure|stop|session-end [--ide NAME]`.
- dir: `tests/hook/fixtures/<ide>/<event>.json` recorded payloads; `<event>.2.json` = same event, 2nd recorded session.
- obs `post-tool-use` body = `<tool_name>\n<tool_input JSON>\n<tool_response JSON>`; `post-tool-use-failure` body = `<tool_name>\n<tool_input JSON>\n<error text>`; JSON compact, keys sorted; both redacted then compressed, identity `tool_use_id` (R5).
- session row (`src/store` T16): `session-start` → `Store::session_started` {`session_id`, ide, `cwd` (absent → empty), event ts}; `session-end` → `Store::session_ended` (`session_id`, event ts).

## §R RESEARCH

id|topic|finding|src
R1|Claude Code dialect, wave 4|fixtures recorded 2026-10-24 by the lead (replace wave 3's documented ones): 2 headless Claude Code 2.1.302 sessions (`claude -p`, same prompt twice, 1 `echo chert-fixture` Bash call), all 5 hooks, 1 file per event per session. sanitized: `cwd` → `/workspace/project`, `transcript_path` → `/workspace/.claude/projects/-workspace-project/<session_id>.jsonl` (recorded under a home dir `user`, moved off it for gate `no-local-paths`); rest byte-for-byte. identity: R4|`tests/hook/fixtures/claude-code/`
R2|token count, wave 4|owner 2026-10-24 "as in itok": default = estimate bytes / 4 rounded up (⊥ dependency, deterministic, ⊥ load). exact o200k behind optional feature `bpe` (off by default): `tiktoken-rs` 0.12.1 MIT, tree MIT / Apache-2.0 / Unlicense OR MIT, deny ok w/ & w/o `bpe`; table loads once per process (~65 ms release, darwin, wave 3 probe); load failure → bytes, ⊥ panic (V2). 1 swap point: `inject.rs` `tokens_at_most`. default since wave 12: R7 (B1)|`src/hook/inject.rs`
R3|choices, wave 3|malformed or store error → `spool/unreadable/<ULID>.hook`, where store sets unreadable spool aside ∴ ⊥ replayed, ⊥ dropped; wave 5 (T8): via `Store::spool_raw` after `<private>` strip; spool_raw fails → exit 0, stderr note "payload lost". injected ref = session's `session-end` obs at summary ts. roll-up joins by newline; gist = 1st line ≤ 200 chars. exclude reads `tool_input` `file_path` `path` `notebook_path`; glob w/o `/` = any component, else from root, dir covers subtree. unclosed `<private>` drops rest. `session-end` opens reading (needs turns)|`src/hook/*.rs`
R4|resends & identity, wave 4|sample = 2 sessions (R1): each event exactly once per session, ⊥ resend seen. `tool_use_id` only on PostToolUse. `prompt_id` on UserPromptSubmit, PostToolUse, Stop, SessionEnd, shared by every event of 1 turn ∴ ⊥ unique per event ∴ ⊥ identity. fields past docs: `prompt_id`, `effort`, `permission_mode`, `duration_ms` (PostToolUse), `background_tasks` & `session_crons` (Stop); ignored (§C). SessionStart `source = startup`; SessionEnd `reason = other` in `-p`; Stop has `last_assistant_message`. ∴ identity stays `tool_use_id` (PostToolUse) only; other events kept every time (`.:V16`). a later recording w/ real resends may amend|`tests/hook/fixtures/claude-code/`
R5|PostToolUseFailure, wave 8|recorded 2026-10-25 by the lead: headless Claude Code 2.1.302 session running `false`, sanitized as R1, 1 session (∴ ⊥ `.2.json`). fields: `tool_name`, `tool_input`, `tool_use_id`, `error` ("Exit code 1"), `is_interrupt`, `duration_ms`, ⊥ `tool_response`; a failed call fires PostToolUseFailure & ⊥ PostToolUse. stored kind `post-tool-use-failure` (= CLI name, as every kind), error text as body line 3; `is_interrupt` & `duration_ms` ignored (§C)|`tests/hook/fixtures/claude-code/post-tool-use-failure.json`
R6|newest partitions first, wave 10|pick walks `partitions` by `t1` desc, `Store::summaries_in` per partition when reached, stops once 3 other sessions picked & oldest pick ts ≥ next `t1` (unread rows < own `t1`) ∴ same pick, (ts, id) ties too (test vs every-partition pick, 4 shards). release, M1 Pro, criterion median ms before → after, `session_start` / `prompt_inject`: 1k-p52 26.2 → 2.6 / 31.2 → 2.7; 10k-p52 26.9 → 2.1 / 26.9 → 2.2; 100k-p52 28.8 → 2.3 / 30.8 → 2.4; 1M-p52 48.7 → 2.7 / 45.8 → 2.8; 100k-p1 14.2 → 3.8 / 14.3 → 3.8; 100k-p4 7.9 → 2.7 / 7.5 → 2.7; 100k-p520 288 → 2.7 / 252 → 2.7. ∀ < 5 ms. bench appends at 1 fixed ms ∴ > ~768 runs per process exhaust that ms's ids (`src/store` R6): run 1 bench per process (`-- '^session_start/1M-p52$'`)|`inject.rs`, `benches/latency/ops/hook.rs`
R7|structured count, wave 12|text split as o200k's pre-split regex (bpe test: Σ o200k(piece) = o200k(text) ∀ corpus text); sixteenths of a token, Σ rounded up. header 12; `- chert:` 4 + 1 per digit run + 1 per ref capital (max: random refs 8–14 chars, ∀ capital runs ≤ 3). word by lead (space+small, space+capital, other) × letters (1, 2–3, 4–5, 6–8, 9–12, 13+): `16 18 18 25 25 25`, `16 16 16 16 37 37`, `23 26 26 26 26 26`; marks by lead (none, space) × marks (1, 2, 3–4, 5+): `20 20 20 20`, `16 16 22 22`; digits, breaks, spaces 16; +8 per non-ASCII byte. fit: integer LP (scipy milp, bisect worst), piece ≥ 16, size-monotone. corpus 1233 texts (`inject/tests/corpus.rs`: README, docs, grammar corpus, fixtures × 4 intensities, 3 lines of 10-char refs; + 5 seeded injections): ⊥ under; over median 1.120, worst 1.227 (3 markdown headings), Σ 130940 / 116944 tok; test caps 1.250. 1.15 unreached: ~0.04 ref bound (exact refs: 1.18), rest common words 1 tok at any length. non-ASCII rare in corpus. recount: `cargo test --features bpe --lib corpus::tests -- --nocapture`|`inject/estimate.rs`

## §V INVARIANTS

V1: `<private>…</private>` removed & `privacy.exclude` paths unread before compress (`.:V8`).
V2: exit 0 ∀ payload, incl. malformed; malformed → spool, ⊥ lost (`.:V2`).
V3: ∀ fixture → exactly 1 observation (or 0 when excluded), same bytes ∀ run (`.:V3`).
V4: `inject.on_prompt` false → `user-prompt-submit` prints nothing & opens store write-only.
V5: injected text ≤ `inject.max_tokens` tokens as counted (R7 estimate, never < o200k on the corpus; exact o200k w/ `bpe`, R2) ∀ fixture & store size; ⊥ full bodies (`.:V17`).
V6: `session-end` summary ≡ concat of first 20 turn summaries of that session, same bytes ∀ run.

## §T TASKS

id|status|task|cites
T1|x|record Claude Code fixtures ∀ 5 events (R1)|V3
T2|x|Claude Code dialect → event struct|V3
T3|x|redaction & exclusion|V1
T4|x|malformed → spool|V2
T5|x|measure: Claude Code resends? event id field per hook? → §R (`.:V16`). R4: 2 sessions, ⊥ resend; later recording w/ real resends may amend|V3
T6|x|injection: session-start default, prompt opt-in, budget cut (R2 count), stdout shape|V4,V5
T7|x|turn summary on stop, session roll-up on session-end|V6
T8|x|unparsable payload via `Store::spool_raw` (`src/store:T14`), ⊥ hook writing the store's dir (owner 2026-10-24)|V2
T9|x|register Claude Code `PostToolUseFailure`: record its payload first (as T1), store failed tool calls as their own kind w/ the error text ∴ `src/mine` T1 sees failures|V3
T10|x|store each session's `cwd`, `started_at` (session-start) & `ended_at` (session-end) for `src/store` T16|V3
T11|x|`session-start` injection < 5 ms in process at 1k–1M rows & 1–520 partitions (benches `session_start`): read newest partitions only until `inject.max_tokens` is filled ⊥ every partition; today 26 ms at 1k, 241 ms at 520 partitions; after `src/store:T19`|V5
T12|x|structured estimate ⊥ flat bytes/4 (B1, owner 2026-10-25): header & each `chert:` ref counted exactly by formula, gist estimated by character class (letters, digits, punctuation, spaces), integer arithmetic; `bpe` test over the committed corpus (1233 texts, R7): estimate ≥ exact ∀ text, report worst over; gate runs `bpe` tests|V5
T13|.|exact counts off the hook path: `chert mcp` counts new rows w/ o200k in background, store keeps a `tokens` column, injection uses stored counts & T12 for the rest; needs `bpe` in default build = owner decision (step 2)|V5

## §B BUGS

id|date|cause|fix
B1|2026-10-25|hook T12 measurement, exact o200k over 1233 injected texts: median 0.34 tokens per byte, max 0.52, but bytes/4 counts 0.25 ∴ injection exceeds `inject.max_tokens` in real tokens by ~37% median, up to ~2× (V5 held only as counted)|T12: R7 count; V5 + bpe test `the_estimate_never_undercounts_o200k_on_the_corpus`
