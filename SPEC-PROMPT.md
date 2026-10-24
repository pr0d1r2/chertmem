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
