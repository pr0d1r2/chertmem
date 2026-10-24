# SPEC

## §G GOAL

crate `chertmem`: lib + bin `chert` (`[[bin]] name = "chert"`, D11). 1 node per module dir; this node holds crate-wide wiring only: `lib.rs`, `main.rs`, error type, module edges.

## §F FEDERATION

dir|owns|⊥owns|tokens
config|`chert.toml` scopes, merge, defaults, validation|reading partitions (`store`), cache sizing (`cache`)|-
store|`catalog.db`, partition files, writes, spool, tiers, refs|ranking (`search`), in-memory rows (`cache`)|-
grammar|caveman compress & expand, ported from cavemem 0.2.1|zstd & storage tiers (`store`)|-
hook|hook intake: event JSON on stdin → redact → append|IDE wiring (`install`), storage (`store`)|-
cache|in-memory recent window in `chert mcp`: load, fit, freshness|disk store (`store`), scoring (`search`)|-
search|BM25 + vector ranking across partitions, one scoring path|row storage (`store`), MCP framing (`mcp`)|-
mcp|MCP stdio server: 4 tools, hosts cache|ranking (`search`), cache internals (`cache`)|-
cli|`chert` verbs via clap, exit codes|verb behaviour (owning modules)|-
install|IDE wiring: hooks & MCP registration, status, doctor|hook payloads (`hook`)|-
import|JSONL export for rekall, import incl. cavemem DB|candidate minting (rekall), mining (`mine`)|-
mine|behaviour findings & awareness report|candidates & rules (rekall), export (`import`)|-

## §N NAV

rel|path|lens
up|.|-
self|src|crate `chertmem`: lib + bin `chert`, 1 node per module
sib|benches|benchmarks: fixtures, suites, baseline, regression gate
sib|dev|root `flake.nix` devShell & `dev` crate (`publish = false`) gate steps
sib|docs|human pages: README, BENCHMARKS, INTEGRATION, CONTRIBUTING, SECURITY, LLM-DISCLAIMER

## §C CONSTRAINTS

- deps minimal & each justified in module node: `rusqlite` (`bundled`), `clap`, `serde`, `serde_json`, `toml`, `zstd`, `ulid`, MCP via `rmcp` | hand-rolled JSON-RPC (decided in `src/mcp`).
- ⊥ async runtime unless a module node proves need; hooks stay sync.
- module edges acyclic (as built, wave 5): `grammar` leaf; `config` → grammar (`Grammar::intensity`); `store` → config; `cache` → config, store; `search` → cache, config, store; `hook` → config, grammar, store; `import`, `mine` → config, grammar, hook, store; `mcp` → search, cache; `cli` top.
- errors: 1 crate error enum; user-facing message `chert: <what> (<fix>)`; exit codes per `.:§I`.

## §I INTERFACES

- lib: `chertmem::{config, store, grammar, hook, cache, search, mcp, cli, install, import, mine}`.
- bin: `chert` → `cli::run`.

## §V INVARIANTS

V1: module graph acyclic, matches §C edge list; test asserts it from `use` paths.
V2: `main.rs` only calls `cli::run` & maps its result to exit code.

## §T TASKS

id|status|task|cites
T1|.|`Cargo.toml` workspace + crate skeleton, `lib.rs` module list, `main.rs`|V2
T2|.|module-edge test|V1

## §B BUGS

id|date|cause|fix
