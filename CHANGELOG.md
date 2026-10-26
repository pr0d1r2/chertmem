# Changelog

All notable changes to chertmem are recorded here, in the
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format. The
project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Version ladder

A minor version here is a level of **guarantee**, not a feature count.
Each rung answers one question: *what can you rely on at this tag?*

| version | what you can rely on | status |
| --- | --- | --- |
| `0.1` | it builds from crates.io on every supported platform, the gate runs in CI, and every verb exists | reached |
| `0.2` | a week of real Claude Code use on the owner's machine without lost events, broken wiring or a cavemem import gap | planned |
| `0.3` | `chert export --for rekall` in the shape agreed in pr0d1r2/rekall#46 | planned |
| `1.0` | the contract frozen: CLI verbs and exit codes, MCP tool shapes, the export format and the on-disk store | planned |

Pre-1.0 SemVer permits a minor to break, and here each rung *is* a
behaviour change, so that permission is used honestly rather than worked
around.

`0.1.0` is the first version for crates.io.

## [Unreleased]

## [0.1.0] - 2026-10-26

First release: a Rust rewrite of cavemem 0.2.1 as one binary, `chert`,
with no Node at run time.

### Added

- Capture: `chert hook run` for Claude Code's 6 hook events, including
  failed tool calls, with `<private>` spans stripped and excluded paths
  skipped; a write takes about 4.5 ms.
- Storage: one SQLite file per project and time bucket under a catalog,
  hot, warm and cold (zstd with a dictionary) tiers, a spool when the
  store is locked, tombstones, and short `chert:<id>` refs that never
  come back once dropped.
- Cache: a rolling in-memory window with a size limit that warns when
  the window does not fit, kept fresh across processes.
- Search: FTS5 candidates per partition, BM25 over the plain and the
  compressed form of each word, an in-memory postings index on the
  cached window, and an optional ollama re-rank.
- Injection: session-start context (prompt injection opt-in) within
  `inject.max_tokens`, counted by a format-aware tokenizer that never
  undercounts o200k.
- MCP: `chert mcp` with cavemem's 4 tools, `search`, `timeline`,
  `get_observations` and `list_sessions`.
- Mining: `chert mine` and `chert report` find failing commands,
  repeated sequences, hot files and token sinks, each with its evidence
  refs.
- `chert install`, `uninstall`, `status` and `doctor` for Claude Code,
  with byte-exact backups.
- `chert import --from cavemem`, JSONL export and import, `chert config`,
  `chert statusline`, completions and a man page.
- Docs: README, CONFIG, INTEGRATION, BENCHMARKS and the generated
  PERFORMANCE page.

[Unreleased]: https://github.com/pr0d1r2/chertmem/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/pr0d1r2/chertmem/releases/tag/v0.1.0
