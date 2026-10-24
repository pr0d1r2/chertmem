DRAFT INPUT. This file is the prompt for the first ck:spec run. When that
run lands SPEC.md, mark this file HISTORICAL INPUT in its own commit; from
then on SPEC.md and the node specs it federates are the law, and where they
differ from this file, they win. Drafted 2026-10-24 by an agent from the
owner's conversation; what the owner decided is marked as a decision, and
what the agent only proposed is an open question.

Paradigm: paradigm (`github.com/pr0d1r2/paradigm`), commit fda10ea on
2026-10-24 (no tag yet). There is no paradigm language page for Rust yet;
the seed accepts `rust`. Process, guardrail roster and shared conventions
come from there; this file states only the product and its deviations.

chertmem

Persistent memory for coding agents, in Rust: hooks capture what each
session did, a partitioned local store keeps it compressed, and agents
retrieve exactly what they need over MCP, fast and correct. It also mines
that history for patterns worth turning into rules and tools.

# Problem and goals

cavemem (npm, `JuliusBrussee/cavemem`) gives Claude Code, Cursor, Gemini
CLI, OpenCode and Codex a shared memory: hooks write observations into
SQLite, compressed with the caveman grammar, and agents query them through
MCP. It runs on Node. Every hook event starts a fresh Node process, its
SQLite binding is a native addon that breaks when the Node ABI changes, and
local embeddings pull in transformers.js and onnxruntime. The fleet wants
those Node dependencies out of the stack.

A rewrite is also the moment to fix the store's shape. cavemem keeps one
database file. Memory grows with every session, while most questions are
about one project and the recent past.

Goals:
- one static binary, no Node and no runtime on PATH; a hook does its work
  and exits within a few milliseconds
- storage partitioned the way TimescaleDB partitions a hypertable, but
  embedded: by project and time bucket, with every partition setting
  configurable, so queries touch only the partitions they need and
  partitions can be managed, moved and replicated one by one
- an in-memory cache of a configurable recent window, bounded by a
  configurable size, that warns when the window does not fit
- targeted retrieval at maximum responsiveness, and never at the cost of
  correctness: a cached answer equals the uncached one
- good compression at rest
- awareness: report what agents do, where tokens and time go, and mine
  repeated patterns into candidate rules, scripts and tools, for rekall to
  turn into enforced checks
- memory implantation: a skill extracted from that history points back
  at its evidence by short refs, expanded only when the skill fires
  (decision 28)

# Naming

- Crate and repository: `chertmem`. Chert is the fine-grained, layered
  stone ancient toolmakers knapped into sharp edges: dense like compressed
  storage, layered like partitions, and quick to cut. The `mem` suffix
  says what it is and nods to cavemem.
- Binary: `chert` (decision 11). The bare `chert` crate is
  taken, so the crate keeps the suffix and `[[bin]] name` sets the command.
- Availability checked 2026-10-24: `chertmem` free on crates.io (API
  returns 404) and npm; no GitHub repository named `chertmem` (0 search
  results), no `pr0d1r2/chertmem`; no Homebrew formula `chert`. Recount:
  `curl https://crates.io/api/v1/crates/chertmem`, `gh search repos
  chertmem`, `gh repo view pr0d1r2/chertmem`, `brew info --formula chert`.
- `chert` on crates.io is `jesopo/chert` 0.2.1, an expression language,
  unchanged since 2024-02-06. crates.io does not force transfers, so the
  bare name needs its owner's consent.
- Rejected alternatives (2026-10-24):
  - `rustmem`: taken on crates.io (`Adaimade/R-Mem` 0.3.0); names the
    language, not the tool.
  - `memcore`: free on crates.io, taken on npm and PyPI; 62 GitHub
    repositories, one of them an 85-star agent-memory project.
  - `chunkmem`: free, but "chunk" reads as RAG text chunking.
  - `blitzmem`: free; says speed only, close to the Blitzmemo app.
  - `memraptor`: free everywhere; says speed, not compression or layers.
  - `memoralis`: free; says nothing of speed or compression, misspelt as
    `memorialis`.
  - `flintknap`, `tallybone`, `petroglyph`: free, weaker fit than chert.
  - `strata`, `midden`, `quipu`, `ochre`, `cairn`, `flint`, `chert`:
    taken on crates.io.

# Survey (2026-10-24)

## cavemem 0.2.1, the tool being replaced

Read from the published tarball. Recount: `npm pack cavemem@0.2.1`, then
grep `dist/*.js` for `.command(`, `CREATE `, `app.get(` and `import(`.

- Repository `JuliusBrussee/cavemem`, MIT, 676 stars, created 2026-04-18,
  last push 2026-08-14 (`gh api repos/JuliusBrussee/cavemem`). Node >= 20.
- 6 runtime dependencies: `commander`, `kleur`, `better-sqlite3` (native
  addon), `hono`, `@hono/node-server`, `@modelcontextprotocol/sdk`.
  Optional: `@xenova/transformers` for local embeddings, through a dynamic
  `import()`; 7 MiniLM, BGE and GTE models listed.
- 5 hook shims (`session-start`, `user-prompt-submit`, `post-tool-use`,
  `stop`, `session-end`), each `exec cavemem hook run <name>`: one Node
  start per agent event.
- CLI verbs: `install`, `uninstall`, `status`, `config show|get|set|open|
  path`, `start`, `stop`, `restart`, `worker`, `viewer`, `doctor`,
  `search`, `compress`, `expand`, `reindex`, `export`, `import`, `reset`,
  `mcp`, `hook run`.
- One SQLite file: tables `sessions`, `observations`, `summaries`,
  `embeddings`, `schema_version`; FTS5 table `observations_fts` kept by 3
  triggers; indexes on session, ts and embedding model.
- 4 MCP tools: `search`, `timeline`, `get_observations`, `list_sessions`.
- Worker and viewer on `127.0.0.1:37777`: `/`, `/sessions/:id`,
  `/api/search`, `/api/sessions`, `/api/sessions/:id/observations`,
  `/api/state`, `/healthz`.
- Bundle: 214573 bytes of JavaScript, 160478 of them in the shared chunk
  that holds the compression grammar, store and ranker.

## Existing Rust agent-memory projects

Recount: `gh api repos/<repo>` for stars and last push,
`curl https://crates.io/api/v1/crates/<crate>` for version and downloads.

repo|stars|crate|last push|shape
asanabrial/leteo|4|leteo 0.4.2, 121 downloads|2026-10-17|MCP + lifecycle hooks, one SQLite file, 22 MCP tools, TUI
edg-l/engram-mcp|18|engram_mcp 0.12.2, 571 downloads|2026-10-18|MCP only, branch-scoped memory, ONNX embeddings, FTS5 + cosine
rajarshighoshal/opencode-memory|1|opencode-memory 0.1.4, 110 downloads|2026-07-05|sqlite-vec, llama.cpp embeddings, RRF fusion
abankowski/mcpmem|0|mcpmem 3.0.7, 270 downloads|2026-10-15|knowledge graph, tree-sitter indexing
dutiona/memory-engine|2|-|2026-07-23|embeddable library: FTS5, vector, graph
OfficialTanishSharma/agentos|1|-|2026-10-09|FTS5 only, 2 tools

Conclusion: none of them is mature (all created in 2026, at most 18
stars), none partitions its store, and none is a drop-in for cavemem: none
implements the caveman grammar or reads cavemem's data. They are reference
code for `rusqlite`, FTS5, `rmcp` and embeddings, not a base to build on.

# Config: `chert.toml`

Two scopes, as rekall has: `~/.config/chert/chert.toml` (user) and
`chert.toml` at the project root (project, tracked). The project scope
overrides the user scope key by key. Sketch, not final:

```toml
[storage]
data_dir = "~/.local/share/chert"

[partition]
by          = ["project", "time"]  # partition dimensions, in order
time_bucket = "7d"                 # 1d | 7d | 1mo | ...
shards      = 1                    # hash sub-partitions per bucket

[cache]
window      = "28d"                # recent window held in memory
max_bytes   = "256MiB"
on_overflow = "warn"               # warn | shrink | fail

[compression]
grammar = "full"                   # caveman grammar: off | lite | full | ultra
zstd    = { level = 3, dictionary = true }

[search]
alpha         = 0.5                # BM25 / vector blend
default_limit = 10

[embedding]
provider = "off"                   # off | local | ollama | openai

[privacy]
exclude = []                       # path globs never captured

[mining]
enabled = true
min_occurrences = 3
```

- `cache.window` should be a whole multiple of `partition.time_bucket`;
  when it is not, chert warns and names both values.
- `<private>...</private>` is stripped before any write, as in cavemem.

# chert CLI

Sketch, not final. Verbs keep cavemem's names where the meaning is the
same, so existing hook shims and MCP registrations change only the
command name.

chert install [--ide NAME] · chert uninstall [--ide NAME] # hooks + MCP per IDE
chert hook run <event> # read the event on stdin, write it, exit
chert mcp # MCP server on stdio; hosts the cache
chert search <query> [--limit N] [--since T] [--project P] [--no-semantic]
chert timeline <session> [--around REF] · chert get <ref>... # ref = chert:<id prefix>, decision 28
chert status # wiring, partitions, cache fit, spool depth
chert doctor # verify wiring; rebuild the catalog from partition files
chert config show|get|set|path
chert partitions [list|seal|compact|drop|repartition] # partition management
chert cache [plan] # what the configured window needs against max_bytes
chert mine [--since T] [--format text|json] # behaviour findings with evidence
chert report [--format text|json|md] # awareness: tools, errors, token sinks
chert compress <file> · chert expand <file>
chert export <out.jsonl> [--for rekall] · chert import <in.jsonl> [--from cavemem]
chert completions <shell> · chert man

- Exit codes: 0 ok, 1 finding, 2 usage or unreadable input.
- MCP tools keep cavemem's names and result shapes: `search`, `timeline`,
  `get_observations`, `list_sessions`.

# Decisions, round 1 (2026-10-24)

From the owner's conversation. Mechanisms the agent proposed but
the owner has not decided are open questions below, not decisions.

1. Rewrite cavemem in Rust to remove its Node dependencies from the
   stack: one binary, no Node at run time, no native addon tied to a
   runtime ABI. Local.
2. Name: crate and repository `chertmem` (Naming). Local.
3. Focus, in this order when they conflict: correctness, then
   responsiveness. Speed, partitionability and targeted retrieval serve
   both; nothing trades a correct answer for a faster one. Local.
4. Storage is partitioned like a TimescaleDB hypertable: data split by
   dimensions (project and time bucket first), queries skip partitions
   that cannot match, and each partition is managed on its own (seal,
   compact, move, replicate, drop). Local.
5. Partitioning is configurable: the dimensions, the time bucket and a
   shard count. The time bucket can match the cache window or divide it,
   so the window is whole partitions; reads gain parallelism from more,
   smaller partitions, and a sharded layout is managed shard by shard. Local.
6. An in-memory cache holds a recent window for the fastest retrieval.
   The window is configurable, not fixed at 30 days. Local.
7. The cache size is configurable. When the configured window does not
   fit in it, chert warns, naming the estimate and the limit. Local.
8. A cached answer equals the uncached answer: the store on disk is the
   source of truth and the cache never serves stale or missing rows. Local.
9. The package is also about awareness and optimization: it reports what
   agents do and mines the history into candidate rules, scripts and
   tools, the way rekall turns prose rules into executable checks. Local.
10. Process: the paradigm lifecycle in full, from fda10ea. Atomic commits,
    one topic each, from this prompt on. Local.

# Decisions, round 2 (2026-10-24)

Gap round 1. The owner told the agent to accept its own proposals for
open questions 1 to 12 and 14 to 17, so the agent answered this round;
each answer stands until the owner amends it by a later decision. Open
question 13 stays open.

11. Binary: `chert`. The crate stays `chertmem` and sets the command with
    `[[bin]] name = "chert"`; hook shims call `exec chert hook run
    <event>`. Closes open question 1. Local.
12. Storage: one SQLite file per partition under `chunks/<project>/`,
    plus `catalog.db` recording each partition's dimensions, bounds, row
    count, state and checksum. The planner routes by the catalog, never
    by the current config, so a config change cannot misroute a read;
    `chert partitions repartition` rewrites old partitions. A partition
    moves from hot (writable, WAL) to warm (sealed, read-only) to cold
    (`VACUUM INTO`, then zstd). Closes open question 2. Local.
13. Writes: ULID ids, unique across partitions and sortable by time; an
    idempotency key hashed from session, event and payload, written with
    `INSERT OR IGNORE`, because IDEs resend events; each observation goes
    to the bucket of its own timestamp, so a session can span buckets; on
    a lock timeout the event goes to an append-only spool that the next
    run replays, so no event is lost. Closes open question 3. Local.
14. Cache host: the long-lived `chert mcp` process of each IDE session
    holds the cache and warms it in a background thread, serving from
    disk until warm. Hooks hold no cache. A shared daemon over a Unix
    socket stays a later option, for when one copy per session costs too
    much. Closes open question 4. Local.
15. Cache freshness (decision 8): `PRAGMA data_version` before each query
    detects commits from other processes; new rows are read by an
    `AUTOINCREMENT` sequence column, not by ULID, because two processes
    writing in the same millisecond can commit out of ULID order; a
    `generation` counter in the catalog, bumped by any delete, redaction
    or rewrite, forces a full reload. Closes open question 5. Local.
16. Cache fit (decision 7): the window's size is estimated from catalog
    statistics before loading. On overflow, `warn` keeps the newest days
    that fit and says how many, `shrink` does the same silently after one
    warning, and `fail` serves from disk only. chert also warns when
    `max_bytes` exceeds about 25% of available memory. Warnings go to the
    MCP log, `chert status` and `chert doctor`, never into the agent's
    context. Closes open question 6. Local.
17. Ranking across partitions: global term statistics kept in the catalog
    (from `fts5vocab`), so BM25 compares across partitions, and one
    scoring path for cached and uncached rows, so a result ranks the same
    wherever it came from. Closes open question 7. Local.
18. Embeddings: off by default; the ollama provider ships in version 1;
    local models come later, through `candle` (pure Rust) or `fastembed`
    (needs ONNX Runtime, a native library). Closes open question 12.
    Local.
19. Defaults: `partition.time_bucket = "7d"`, `cache.window = "28d"`,
    `cache.max_bytes = "256MiB"`, `partition.shards = 1`, to be checked
    against a real month of history. Closes open question 8. Local.
20. Responsiveness targets, measured by benchmarks: hook write under
    5 ms, search of the recent window under 10 ms, `get_observations`
    under 2 ms, semantic re-rank under 20 ms. A benchmark step in the
    push gate fails on a regression. Closes open question 9. Local.
21. cavemem compatibility: chert keeps its own data directory and imports
    cavemem's database with `chert import --from cavemem`; it never shares
    cavemem's live files. MCP tool names and result shapes stay
    compatible. Closes open question 10. Local.
22. Caveman grammar: port cavemem's compress and expand at a pinned
    cavemem version, byte for byte, proved by differential tests that run
    cavemem's JavaScript as the oracle over a corpus. Node is a test-only
    dependency of that step, never a run-time one (decision 1). cavemem
    is MIT, so its copyright notice ships with the port. Closes open
    question 11. Local.
23. Mining signals in version 1: repeated failing commands and errors,
    repeated command sequences, user corrections, and hot files and token
    sinks. Candidates are deterministic (same partitions, same
    candidates), carry observation ids as evidence, and stay rejected
    once rejected. Each sealed partition is mined once. Closes open
    question 14. Local. Aligned by decision 27: user corrections are
    left to `rekall catch`, and candidate state lives in rekall's ledger,
    not in chert.
24. IDE coverage: Claude Code in version 1; Cursor, Gemini CLI, OpenCode
    and Codex once each has recorded hook-payload fixtures. Closes open
    question 15. Local.
25. Viewer: no web viewer in version 1; `chert report` and `chert status`
    cover awareness. Closes open question 16. Local.
26. Visibility: public on GitHub from the first push, MIT, a fleet member
    whose CI is rendered by actlith, like the other fleet tools. Closes
    open question 17. Local.

# Decisions, round 3 (2026-10-24)

Gap round 2, answered by the owner, after reading rekall
(`github.com/pr0d1r2/rekall`, commit 3235597). rekall stores a candidate
as `{id, src, text, at}` in `.rekall/ledger.toml` (`src/ledger/mod.rs`),
mints candidates only through its own classifier (`src/catch` V44) and
rejects a second store for them (`src/catch` V45).

27. rekall reads chert; chert never writes `.rekall/ledger.toml`. chert
    exports observations as JSONL, and rekall reads them as one more
    transcript root, through a `chert` dialect in `rekall catch`. rekall's
    classifier stays the only one that mints candidates. User corrections
    are rekall's signal; chert keeps the behaviour signals of decision 23
    (failing commands, repeated sequences, hot files, token sinks) and
    reports them through `chert mine` and `chert report`. The `chert`
    dialect is a rekall change, proposed there by an issue only with the
    owner's approval. Closes open question 13. Local.

# Decisions, round 4 (2026-10-24)

Gap round 3, answered by the owner: memory implantation, raised by the
owner during the spec run.

28. Memory implantation: a skill or rule extracted from chert's history
    points back at its evidence for the fewest tokens. Each observation
    has a stable ref `chert:<id>`, written as the shortest ULID prefix
    unique within its project, at least 8 characters, as git shortens
    hashes. Refs ride in chert's export into rekall's candidate `src`
    (decision 27), so the artifact rekall writes carries refs only, about
    5 tokens each, never copied evidence. When the skill fires, the agent
    expands a ref with `chert get <ref>` or MCP `get_observations`, which
    returns the compressed body; evidence costs tokens only when read. A
    ref resolves for as long as its observation exists, across
    repartition and compaction, because the catalog routes by id
    (decision 12); a ref whose observation is gone resolves to a clear
    "dropped" answer, never to another observation. Local.

# Decisions, round 5 (2026-10-24)

Gap round 4, answered by the owner during the spec run.

29. Benchmarking is a first-class part of the package, with human docs
    on it. Benchmarks cover every path decision 20 sets a target for,
    plus cache warm-up and partition scans at several sizes; they run on
    generated fixtures of stated size, so a result can be reproduced; a
    human page explains how to run them, how to read the results, what
    the targets are and why, and the measured numbers, each with its
    date, machine and recount command (paradigm/V4). Decision 20's gate
    step reads the same benchmarks. Local.

# Non-goals

Proposed by the agent; the owner has not ruled on them yet.

- No server to run: no Postgres, no required daemon. A daemon, if any, is
  an accelerator that every path works without.
- No cloud and no network by default. Remote embedding providers are
  opt-in, and nothing else leaves the machine.
- No automatic changes from mining: chert reports findings with
  evidence, rekall mints candidates from chert's export (decision 27), a
  human accepts them, and rekall enforces them.
- No agent harness features: chert does not schedule agents or manage
  their usage limits.
- No general-purpose time-series database: partitions serve agent memory
  only.

# Prior art and rejected designs

Proposed by the agent; each rejection stands until the owner reopens it.

- TimescaleDB itself: rejected as the default store. It is a Postgres
  extension, so it needs a running server, an install, upgrades and a
  connection per hook. Its chunking and chunk exclusion are the part
  chert takes. It can return later as an optional backend for shared
  team memory behind a storage trait.
- One SQLite file for everything (cavemem): rejected. Every query and
  every maintenance step touches the whole history.
- Embedded stores owned by one process (redb, fjall, sled): rejected for
  the shared store. Hooks, several MCP servers and a worker write at the
  same time from separate processes; SQLite in WAL mode allows that.
- DuckDB: rejected. One read-write process at a time, and its strength is
  analytics, not small point writes.
- `ATTACH` of every partition into one connection: rejected. SQLite
  allows 10 attached databases by default; one connection per partition,
  queried in parallel, has no such limit.
- Merging raw BM25 scores across partitions: rejected. Each partition has
  its own term statistics, so raw scores do not compare.
- Building on an existing Rust memory server (Survey): rejected. None
  partitions, none is mature, and none is compatible with cavemem.
- Differentiator: cavemem's capture and compression, with a store
  partitioned for fast targeted reads, a bounded cache that is never
  stale, and mining that feeds rekall.

# Open questions

Each carries the agent's proposal. A gap round turns each one into a
decision or drops it.

1. Binary name. Closed by decision 11.
2. Storage engine and layout. Closed by decision 12.
3. Write correctness. Closed by decision 13.
4. Cache host. Closed by decision 14.
5. Cache freshness. Closed by decision 15.
6. Cache fit. Closed by decision 16.
7. Ranking across partitions. Closed by decision 17.
8. Defaults. Closed by decision 19.
9. Responsiveness targets. Closed by decision 20.
10. cavemem compatibility. Closed by decision 21.
11. Caveman grammar. Closed by decision 22.
12. Embeddings in version 1. Closed by decision 18.
13. Mining hand-off to rekall. Closed by decision 27.
14. Mining signals in version 1. Closed by decision 23.
15. IDE coverage in version 1. Closed by decision 24.
16. Viewer. Closed by decision 25.
17. Visibility, license and fleet membership. Closed by decision 26.
