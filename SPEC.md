# SPEC

## §G GOAL

`chertmem` — persistent memory for coding agents, 1 static Rust binary `chert`, ⊥ Node at run time. hooks capture session events → partitioned local store (SQLite file per project × time bucket, TimescaleDB-style chunk exclusion) keeps them compressed (caveman grammar + zstd) → agents retrieve targeted rows over MCP, fast & correct: cached answer ≡ uncached answer. also: awareness report & behaviour mining, exported for rekall; extracted skills point back by short `chert:<id>` refs.

LINEAGE: `SPEC-PROMPT.md` (HISTORICAL after this run). where it & this spec differ, this spec wins. prompt decisions cited `D<n>`. paradigm `fda10ea`, no Rust language page yet; fleet rules cited `paradigm/V<n>`.

## §D DESCRIPTION

Rust cargo workspace: crate `chertmem` (lib + bin `chert`, MIT, public) & `dev` (`publish = false`, gate steps). rewrite of cavemem 0.2.1 (npm, `JuliusBrussee/cavemem`, MIT): same capture idea & MCP tool names, own data dir, import path from cavemem. store = `catalog.db` + 1 SQLite file per partition; long-lived `chert mcp` holds bounded in-memory cache of recent window. hooks = short processes, write & exit. mining reports behaviour findings; rekall reads chert's JSONL export & alone mints candidates (D27). fleet member, full paradigm conformance (D10).

## §F FEDERATION

dir|owns|⊥owns|tokens
