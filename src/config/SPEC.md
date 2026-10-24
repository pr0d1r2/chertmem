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

## §C CONSTRAINTS

- format TOML via `toml` + `serde`. unknown key → error naming key & file, ⊥ ignored.
- sizes parse `KiB` `MiB` `GiB`; durations parse `<n>d` `<n>h` `<n>mo`.
- ⊥ I/O beyond reading the 2 files.

## §I INTERFACES

- fn: `config::load(project_root) -> Result<Config>`.
- file: `~/.config/chert/chert.toml`, `<project>/chert.toml`; keys: `storage.data_dir`, `partition.{by,time_bucket,shards}`, `cache.{window,max_bytes,on_overflow}`, `compression.{grammar,zstd}`, `search.{alpha,default_limit}`, `embedding.provider`, `privacy.exclude`, `mining.{enabled,min_occurrences}`.

## §V INVARIANTS

V1: project value overrides user value per key; absent key → default from `.:§C`.
V2: `cache.window` ⊥ whole multiple of `partition.time_bucket` → warning naming both values; load still succeeds (D5).
V3: invalid value → error w/ file, key, value & accepted forms; ⊥ silent default.

## §T TASKS

id|status|task|cites
T1|.|`Config` type, parse & merge|V1,V3
T2|.|window/bucket multiple warning|V2

## §B BUGS

id|date|cause|fix
