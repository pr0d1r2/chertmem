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
