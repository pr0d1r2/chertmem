# SPEC

## §G GOAL

`chertmem` — persistent memory for coding agents, 1 static Rust binary `chert`, ⊥ Node at run time. hooks capture session events → partitioned local store (SQLite file per project × time bucket, TimescaleDB-style chunk exclusion) keeps them compressed (caveman grammar + zstd) → agents retrieve targeted rows over MCP, fast & correct: cached answer ≡ uncached answer. also: awareness report & behaviour mining, exported for rekall; extracted skills point back by short `chert:<id>` refs.

LINEAGE: `SPEC-PROMPT.md` (HISTORICAL after this run). where it & this spec differ, this spec wins. prompt decisions cited `D<n>`. paradigm `fda10ea`, no Rust language page yet; fleet rules cited `paradigm/V<n>`.
