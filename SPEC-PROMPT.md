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

# Naming

- Crate and repository: `chertmem`. Chert is the fine-grained, layered
  stone ancient toolmakers knapped into sharp edges: dense like compressed
  storage, layered like partitions, and quick to cut. The `mem` suffix
  says what it is and nods to cavemem.
- Binary: `chert` is proposed (open question). The bare `chert` crate is
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
chert timeline <session> [--around ID] · chert get <id>...
chert status # wiring, partitions, cache fit, spool depth
chert doctor # verify wiring; rebuild the catalog from partition files
chert config show|get|set|path
chert partitions [list|seal|compact|drop|repartition] # partition management
chert cache [plan] # what the configured window needs against max_bytes
chert mine [--since T] [--format text|json] # candidates with evidence
chert candidates [list|accept|reject] <id>
chert report [--format text|json|md] # awareness: tools, errors, token sinks
chert compress <file> · chert expand <file>
chert export <out.jsonl> · chert import <in.jsonl> [--from cavemem]
chert completions <shell> · chert man

- Exit codes: 0 ok, 1 finding, 2 usage or unreadable input.
- MCP tools keep cavemem's names and result shapes: `search`, `timeline`,
  `get_observations`, `list_sessions`.

# Decisions (2026-10-24)

Round 1, from the owner's conversation. Mechanisms the agent proposed but
the owner has not decided are open questions below, not decisions.

1. Rewrite cavemem in Rust to remove its Node dependencies from the
   stack: one binary, no Node at run time, no native addon tied to a
   runtime ABI.
2. Name: crate and repository `chertmem` (Naming).
3. Focus, in this order when they conflict: correctness, then
   responsiveness. Speed, partitionability and targeted retrieval serve
   both; nothing trades a correct answer for a faster one.
4. Storage is partitioned like a TimescaleDB hypertable: data split by
   dimensions (project and time bucket first), queries skip partitions
   that cannot match, and each partition is managed on its own (seal,
   compact, move, replicate, drop).
5. Partitioning is configurable: the dimensions, the time bucket and a
   shard count. The time bucket can match the cache window or divide it,
   so the window is whole partitions; reads gain parallelism from more,
   smaller partitions, and a sharded layout is managed shard by shard.
6. An in-memory cache holds a recent window for the fastest retrieval.
   The window is configurable, not fixed at 30 days.
7. The cache size is configurable. When the configured window does not
   fit in it, chert warns, naming the estimate and the limit.
8. A cached answer equals the uncached answer: the store on disk is the
   source of truth and the cache never serves stale or missing rows.
9. The package is also about awareness and optimization: it reports what
   agents do and mines the history into candidate rules, scripts and
   tools, the way rekall turns prose rules into executable checks.
10. Process: the paradigm lifecycle in full, from fda10ea. Atomic commits,
    one topic each, from this prompt on.

# Non-goals

Proposed by the agent; the owner has not ruled on them yet.

- No server to run: no Postgres, no required daemon. A daemon, if any, is
  an accelerator that every path works without.
- No cloud and no network by default. Remote embedding providers are
  opt-in, and nothing else leaves the machine.
- No automatic changes from mining: chert proposes candidates with
  evidence, a human accepts them, and rekall enforces them.
- No agent harness features: chert does not schedule agents or manage
  their usage limits.
- No general-purpose time-series database: partitions serve agent memory
  only.
