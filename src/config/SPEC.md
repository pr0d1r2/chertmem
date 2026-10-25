# SPEC

## §G GOAL

load `chert.toml` from user & project scope, merge key by key (project wins), apply defaults (`.:§C`, D19), validate. 1 typed `Config` for every other module.

## §N NAV

rel|path|lens
up|.|-
up|src|crate `chertmem`: lib + bin `chert`, 1 node per module
self|src/config|`chert.toml` scopes, merge, defaults, validation
sib|src/store|`catalog.db`, partition files, writes, spool, tiers, refs
sib|src/grammar|caveman compress & expand, ported from cavemem 0.2.1
sib|src/hook|hook intake: event JSON on stdin → redact → append
sib|src/cache|in-memory recent window in `chert mcp`: load, fit, freshness
sib|src/search|BM25 + vector ranking across partitions, one scoring path
sib|src/mcp|MCP stdio server: 4 tools, hosts cache
sib|src/cli|`chert` verbs via clap, exit codes
sib|src/install|IDE wiring: hooks & MCP registration, status, doctor
sib|src/import|JSONL export for rekall, import incl. cavemem DB
sib|src/mine|behaviour findings & awareness report

## §C CONSTRAINTS

- format TOML via `toml` + `serde`. unknown key → error naming key & file, ⊥ ignored.
- `partition.by` ⊆ {`project`, `time`, `ide`}, ordered, ∋ `time` ∵ window & tiers need time buckets; other value → error (V3).
- sizes parse `KiB` `MiB` `GiB`; durations parse `<n>d` `<n>h` `<n>mo`.
- ⊥ I/O beyond reading the 2 files.

## §I INTERFACES

- fn: `config::load(project_root) -> Result<Loaded, Error>`, `Loaded { config, warnings }` (V2 warnings as data); `config::load_from(home: Option<&Path>, project_root)` w/ home injected, `None` skips user file. `config::Error` ∈ Read Parse UnknownKey Invalid, each naming file, message `chert: <what> (<fix>)`; local until `src` crate error enum lands.
- file: `~/.config/chert/chert.toml`, `<project>/chert.toml`; keys: `storage.data_dir`, `partition.{by,time_bucket,shards}`, `cache.{window,max_bytes,on_overflow}`, `compression.grammar`, `compression.zstd.{level,dictionary}` (`zstd` = table, ⊥ scalar), `search.{alpha,default_limit}`, `embedding.provider`, `privacy.exclude`, `mining.{enabled,min_occurrences}`, `inject.{max_tokens,on_prompt}` (default 300, false).

## §R RESEARCH

id|topic|finding|src
R1|zstd levels|`compression.zstd.level` accepts 1..=22: zstd's positive levels, 20..=22 = `--ultra`; 0 (alias of default 3) & negative fast levels rejected, each 1 more spelling ⊥ asked by root §C|zstd manual: `ZSTD_minCLevel()` < 0, `ZSTD_maxCLevel()` = 22
R2|byte-kept set|`toml_edit` 0.25 (toml's parser & writer) keeps decor & order of untouched items; new section → `[section]`, in an inline table stays inline|toml_edit `DocumentMut`

## §V INVARIANTS

V1: project value overrides user value per key; absent key → default from `.:§C`.
V2: `cache.window` ⊥ whole multiple of `partition.time_bucket` → warning naming both values; load still succeeds (D5).
V3: invalid value → error w/ file, key, value & accepted forms; ⊥ silent default.
V4: setting `inject.on_prompt = true` via `chert config set` prints speed disclaimer once: ∀ prompt then waits for a store read & spool replay; cost per `docs/BENCHMARKS.md`.
V5: accepted values (as built): `on_overflow` ∈ warn shrink fail · `grammar` ∈ off lite full ultra · `zstd.level` ∈ 1..=22 (R1), `zstd.dictionary` bool, each table key merged & defaulted alone · `provider` ∈ off ollama openai · `alpha` ∈ [0,1] · `shards`, `default_limit`, `min_occurrences` ≥ 1 · `max_tokens` ≥ 0 · sizes & durations n ≥ 1 · `partition.by` distinct · `privacy.exclude` string array, default empty · `storage.data_dir` default `<home>/.local/share/chert`. window & bucket: h & d compared in hours, `mo` only against `mo` ∴ mixed → warning.

## §T TASKS

id|status|task|cites
T1|x|`Config` type, parse & merge|V1,V3
T2|x|window/bucket multiple warning|V2
T3|x|`inject.*` keys & on_prompt disclaimer|V4
T4|x|defaults for `compression.*`, `search.*`, `mining.*` per `.:§C`; `compression.zstd` = table `{ level, dictionary }`, ⊥ bool|V1,V5
T5|x|split `config.rs` (5158 tok) & its tests (4214) under the 4000-tok file budget of `sherd check`: 1 submodule per section|-
T6|x|API for `chert config` show, get, set, path: effective config w/ the file each value came from, get 1 key, set 1 key in the project or user file keeping other bytes, file paths (src/cli report)|V1,V3

## §B BUGS

id|date|cause|fix
