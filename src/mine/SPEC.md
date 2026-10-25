# SPEC

## §G GOAL

`chert mine` & `chert report`: behaviour findings w/ evidence refs — repeated failing commands & errors, repeated command sequences, hot files, token sinks (D23, D27). user corrections left to rekall.

## §N NAV

rel|path|lens
up|.|-
up|src|crate `chertmem`: lib + bin `chert`, 1 node per module
self|src/mine|behaviour findings & awareness report
sib|src/config|`chert.toml` scopes, merge, defaults, validation
sib|src/store|`catalog.db`, partition files, writes, spool, tiers, refs
sib|src/grammar|caveman compress & expand, ported from cavemem 0.2.1
sib|src/hook|hook intake: event JSON on stdin → redact → append
sib|src/cache|in-memory recent window in `chert mcp`: load, fit, freshness
sib|src/search|BM25 + vector ranking across partitions, one scoring path
sib|src/mcp|MCP stdio server: 4 tools, hosts cache
sib|src/cli|`chert` verbs via clap, exit codes
sib|src/install|IDE wiring: hooks & MCP registration, status, doctor
sib|src/import|JSONL export for rekall, import incl. cavemem DB

## §C CONSTRAINTS

- mine each sealed partition once; findings stored per partition; hot partition mined on demand.
- threshold `mining.min_occurrences`.
- ⊥ change anything outside chert's data dir (`.:§C`).

## §I INTERFACES

- cmd: `chert mine [--since T] [--format text|json]`; `chert report [--format text|json|md]`.
- finding: `{id, kind, count, refs[], summary}`; `id` = hash of pattern.

## §R RESEARCH

id|topic|finding|src
R1|finding store, wave 5|own SQLite file `<data_dir>/mine/findings.db` (WAL, 1 s busy), ⊥ `catalog.db` & partition files (`src/store`'s). key = partition path + project + checksum + `min_occurrences` + version (`<crate version>/signals-3`, bumped w/ every signal change, wave 6: token sinks, wave 8: failures); ⊥ `generation`: catalog-wide, bumped by any drop \| rewrite anywhere ∴ would re-mine all; checksum moves iff partition's rows do. hot → mined each call, ⊥ kept. dropped partition ⊥ listed ∴ its kept findings ⊥ returned (V2). rows of paths gone by repartition stay orphaned, harmless|`src/mine/ledger.rs`
R2|failing commands, wave 5, reconcile 8|stored `post-tool-use` body = `name\n{tool_input}\n{tool_response}` (`src/hook`); Claude Code Bash `tool_response` = `stdout stderr interrupted isImage noOutputExpected`, ⊥ exit status ∴ ⊥ guess from stderr text. wave 8: failures hooked (`src/hook` T9, R5): a failed call fires `PostToolUseFailure` & ⊥ `PostToolUse`, stored as kind `post-tool-use-failure`, body `name\n{tool_input}\n<error text>` (Bash: `Exit code N`) → R6|`tests/hook/fixtures/claude-code/post-tool-use-failure.json`
R3|token sinks, wave 5|count ! = `src/hook` estimate (bytes/4, o200k w/ `bpe`); `tokens_at_most` private in `src/hook/inject.rs` ∴ token sinks wait on it as `pub(crate)` \| shared fn. reconcile 5: `pub(crate)` → R5|`src/hook/inject.rs`
R4|signals as built, wave 5|sequence = 2 consecutive Bash commands of 1 session in 1 partition, ref = 1st's row; hot file = path in `file_path`, `path`, `notebook_path`, 1 per call. threshold per partition, then merged by id (count = Σ, refs in partition order) ∴ pattern under threshold in every partition ⊥ found. `--since` prunes by partition. commands as stored, compressed at `compression.grammar`: same command → same text; grammar drops leading `a` of path `a/b` (`a/b.ipynb` → `/b.ipynb`)|`src/mine/signal.rs`, `src/mine/tool.rs`
R5|token sinks, wave 6|cost of 1 call = `tokens_at_most` over stored (compressed) body, as `src/hook` counts budget (R3). pattern = tool name + Bash command, else + paths it names, else name alone (⊥ query etc.). sink = pattern w/ Σ cost ≥ 1/10 of Σ cost of all tool calls in partition, & ≥ `min_occurrences` calls; refs = its calls' rows, time order. share per partition, then merged by id as R4 ∴ summary `token sink <pattern>: >= 1/10 of tool call tokens` true of every partition merged; ⊥ token total in summary (merge sums refs only)|`src/mine/signal.rs`
R6|failures, wave 8|kind `failure`, from `post-tool-use-failure` rows only (⊥ feed sequences, hot files, sinks; successes ⊥ failures). pattern = call pattern of R5 + error = 1st non-blank line of body line 3 on, trimmed: later lines = output varying run to run, 1st = stable cause; numbers kept (exit code tells failures apart, number in a path names another path). error as stored, compressed (R4). threshold per partition, merged by id as R4, refs time order. summary `failing <call>: <error>`|`src/mine/signal.rs`, `src/mine/tool.rs`

## §V INVARIANTS

V1: same partitions & version → same findings, same ids, same order (`.:V9`).
V2: ∀ finding carries ≥ `min_occurrences` refs, each resolvable (`.:V5`).
V3: findings ⊥ written under `.rekall/` (`.:V7`).

## §T TASKS

id|status|task|cites
T1|x|failing-command & error signal from `post-tool-use-failure` rows (R2, R6)|V1,V2
T2|x|command-sequence signal (R4)|V1,V2
T3|x|hot files & token sinks, `report` (R3, R4, R5)|V1,V2
T4|x|per-partition finding store (R1)|V1

## §B BUGS

id|date|cause|fix
