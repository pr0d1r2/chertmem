# Configuration

chert runs with no configuration file at all: every key below has a
default, and the defaults are what most people want. A `chert.toml`
only changes what it names.

This page lists every key chert accepts. A test in `src/config` reads
this page and fails when a key is missing here, when the page names a
key chert does not know, or when a default here differs from the one
chert is built with.

## Where the files live

chert reads two files and merges them key by key:

- the user file, `~/.config/chert/chert.toml`, for every project;
- the project file, `chert.toml` in the project's root directory.

A key set in the project file wins over the same key in the user file;
a key set in neither takes its default. `chert config path` prints both
paths.

```sh
chert config show                       # every key, its value, and where it came from
chert config get cache.window           # one key
chert config set cache.window 14d       # write it into the project file
chert config set --user cache.window 14d  # or into the user file
```

`chert config set` rewrites only the key it sets and keeps every other
byte of the file, comments included.

A key chert does not know is an error that names the key and the file,
not something chert skips over. So is a value outside the accepted
forms: the message names the file, the key, the value and what is
accepted, and chert exits with status 2. chert never falls back to a
default in silence.

## Value forms

- **Duration**: `"<n>h"`, `"<n>d"` or `"<n>mo"` with n at least 1, for
  hours, days and months: `"7d"`, `"12h"`, `"1mo"`.
- **Size**: `"<n>KiB"`, `"<n>MiB"` or `"<n>GiB"` with n at least 1:
  `"256MiB"`.
- **Path**: a string. chert does not expand `~` in a path you write;
  give the full path.

## Keys

The Default column is the value as `chert config show` prints it when
no file sets the key; `~` there stands for your home directory.

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `storage.data_dir` | path | `"~/.local/share/chert"` | Where chert keeps its store: `catalog.db`, the partition files under `chunks/`, the `spool/` and its own indexes. |
| `partition.by` | array of `"project"`, `"time"`, `"ide"`, distinct, holding `"time"` | `["project", "time"]` | The dimensions that split the store into partition files, in this order. `"time"` is required, because the cache window and the hot, warm and cold tiers work on time buckets. |
| `partition.time_bucket` | duration | `"7d"` | How much time one partition file covers. |
| `partition.shards` | integer, at least 1 | `1` | How many files each bucket is split into; an observation's shard is a hash of its project and id. |
| `cache.window` | duration | `"28d"` | How far back `chert mcp` keeps the current project in memory. A window that is not a whole number of `partition.time_bucket`s still loads, with a warning naming both values. |
| `cache.max_bytes` | size | `"256MiB"` | The most memory the cache may take. chert estimates the window's size from the catalog before loading it. |
| `cache.on_overflow` | `"warn"`, `"shrink"` or `"fail"` | `"warn"` | What happens when the window would not fit in `cache.max_bytes`. `warn` keeps the newest days that fit and says how many, on every load; `shrink` does the same and warns once; `fail` keeps nothing in memory, so every query reads from disk. |
| `compression.grammar` | `"off"`, `"lite"`, `"full"` or `"ultra"` | `"full"` | How hard the caveman grammar compresses stored text. `off` stores text as captured; each step up drops more filler words and abbreviates more. Code, URLs, paths, identifiers and numbers are never changed. Search compresses the query the same way, so the two always meet. |
| `compression.zstd` | table of the two keys below | - | zstd settings for cold partitions. Write it as a table, `[compression.zstd]` or `compression.zstd = { level = 3 }`, not as a single value; each key in it is merged and defaulted on its own. |
| `compression.zstd.level` | integer, 1 to 22 | `3` | The zstd level `chert partitions compact` packs a cold partition with; 20 to 22 are zstd's `--ultra` levels. |
| `compression.zstd.dictionary` | `true` or `false` | `true` | Train a zstd dictionary for each partition when it is compacted, and store it beside the packed file. |
| `search.alpha` | number, 0 to 1 | `0.5` | How much the vector score counts against the keyword (BM25) score when embeddings are on: 0 is keyword only, 1 is vector only. |
| `search.default_limit` | integer, at least 1 | `10` | How many hits a search returns when it is not given a limit. |
| `embedding.provider` | `"off"`, `"ollama"` or `"openai"` | `"off"` | Where embeddings for semantic re-ranking come from. `off` sends nothing anywhere. `ollama` calls the Ollama server that `OLLAMA_HOST` names (default `127.0.0.1:11434`); it is chert's only network use, and only when you set it. `openai` is accepted but not built yet: search stays keyword-only and logs one warning. |
| `privacy.exclude` | array of path globs | `[]` | Paths whose tool calls chert never captures: a call naming a matching `file_path`, `path` or `notebook_path` is not stored at all. A glob without `/` matches any one path component (`.env`, `*.pem`); one with `/` matches from the project root, and a directory covers everything under it. `*` and `?` stay within a component; `**` crosses them. |
| `mining.enabled` | `true` or `false` | `true` | Whether `chert mine` and `chert report` look for behaviour findings. With `false` they find nothing and write nothing. |
| `mining.min_occurrences` | integer, at least 1 | `3` | How many times a pattern must repeat within a partition before it becomes a finding. |
| `inject.max_tokens` | integer, at least 0 | `300` | The budget for the context chert hands the agent at session start, in estimated tokens (bytes divided by 4; exact o200k tokens in a build with the `bpe` feature). The text is references and one-line gists, never whole bodies. |
| `inject.on_prompt` | `true` or `false` | `false` | Also inject context on every prompt, not only at session start. Read the note below before turning it on. |

## Before you turn on `inject.on_prompt`

With `inject.on_prompt = true`, every prompt you send waits for chert
to replay its spool and read the store before the agent sees the
prompt. With it off, the prompt hook only writes, which is chert's
fastest path. `chert config set inject.on_prompt true` prints this
warning once when it turns the key on; set it back to `false` to undo.

The measured cost will be in [Benchmarks](BENCHMARKS.md).
TODO(docs T2): state the measured per-prompt cost here once the first
benchmark baseline exists.

## An example

```toml
# chert.toml in a project root
[partition]
time_bucket = "1d"

[cache]
window = "14d"

[privacy]
exclude = [".env", "*.pem", "config/credentials.yml"]

[compression.zstd]
level = 9
```
