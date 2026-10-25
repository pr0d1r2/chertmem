# Integration

How chert plugs into a coding agent, what it changes on your machine,
and how data gets in and out. Each step can be undone.

## What you need

- `chert` installed where it will stay (see [Install](../README.md#install)).
  The hooks and the MCP entry call it by its full path, so install it
  first and wire it second.
- An IDE chert knows. v1 knows one: Claude Code. Other IDEs are added
  once their hook payloads have been recorded and tested, so chert
  never guesses at a payload's shape.

## Claude Code

```sh
chert install    # same as chert install --ide claude-code
chert status
```

`chert install` changes two files in your home directory, and nothing
else.

`~/.claude/settings.json` gets one hook group per event, appended after
any hooks you already have, never replacing them:

| Claude Code event | Command chert adds |
| --- | --- |
| `SessionStart` | `chert hook run session-start --ide claude-code` |
| `UserPromptSubmit` | `chert hook run user-prompt-submit --ide claude-code` |
| `PostToolUse` | `chert hook run post-tool-use --ide claude-code` |
| `PostToolUseFailure` | `chert hook run post-tool-use-failure --ide claude-code` |
| `Stop` | `chert hook run stop --ide claude-code` |
| `SessionEnd` | `chert hook run session-end --ide claude-code` |

`~/.claude.json` gets the MCP server, in the shape
`claude mcp add --scope user` writes:

```json
{
  "mcpServers": {
    "chert": {
      "args": ["mcp"],
      "command": "<full path to chert>",
      "env": {},
      "type": "stdio"
    }
  }
}
```

Before its first change to either file, chert copies it to
`<file>.chert.bak`. Every other key is kept; the file is written back
with sorted keys and two-space indentation, and keeps its permissions
(`~/.claude.json` is private), which its backup gets too.
Running `chert install` again reports `unchanged`.

`chert status` lists each entry as `ok` or `missing (run chert install)`.
`chert doctor` checks the same wiring, rebuilds the catalog from the
partition files and compares it, and lists spooled events that were
never stored; it exits 1 when it finds anything.

`chert uninstall` removes only the entries chert wrote. When what is
left matches the backup, the backup's bytes are put back; when the file
did not exist before chert and nothing else is left in it, it is
removed; otherwise the rest is written and the backup kept, so you can
compare.

### What each hook does

Every hook reads one event as JSON on standard input, writes once and
exits, with status 0 even for a payload it cannot parse: that payload
is kept aside under `spool/unreadable/` in the data directory, not
lost. A hook never exits 2, because Claude Code treats exit 2 as
"block the agent".

- `post-tool-use` stores the tool's name, input and output;
  `post-tool-use-failure` stores the name, input and error text.
- `user-prompt-submit` stores the prompt.
- `stop` stores a turn summary from the agent's last message;
  `session-end` stores a session summary made of the first 20 turn
  summaries.
- `session-start` records the session (its id, the IDE and the working
  directory) and, for a fresh start (not a resume or a compaction),
  prints context for the agent: the last 3 other sessions' summaries
  as one-line gists with their refs, then the top findings, cut at
  `inject.max_tokens`.

Context on every prompt is off by default; `inject.on_prompt` turns it
on, at a cost to every prompt ([Configuration](CONFIG.md)).

`<private>…</private>` spans are removed and `privacy.exclude` paths
skipped before any of this is written.

## The MCP tools

`chert mcp` speaks MCP over standard input and output. Claude Code
starts it for each session; it logs to standard error only. The tool
names, parameters and result fields are cavemem 0.2.1's, so an agent
prompt written for cavemem works unchanged, with one deliberate
difference: `get_observations` returns the compressed body unless you
ask for `expand: true` (cavemem expands by default), because the
compressed form costs fewer tokens.

| Tool | Parameters | Returns |
| --- | --- | --- |
| `search` | `query`, `limit?` (1 to 50) | `[{id, score, snippet, session_id, ts}]`, best first |
| `timeline` | `session_id`, `around_id?`, `limit?` (1 to 200) | `[{id, kind, ts}]` in order |
| `get_observations` | `ids` (1 to 50 refs), `expand?` | the bodies; a gone ref as `{id, dropped: true}` |
| `list_sessions` | `limit?` (1 to 200, default 20) | `[{id, ide, cwd, started_at, ended_at}]`, newest first |

Every `id` of an observation is its `chert:<id>` ref. `search` and
`list_sessions` cover the current project.

## Statusline badge

`chert statusline` prints a one-line badge for Claude Code's status
line, modeled after [itok](https://github.com/pr0d1r2/itok)'s
`itok rate --statusline`:

```text
chert:3ms,13k,17.3MB
```

- `3ms` is the median reply time of the last 20 replies of the
  project's running `chert mcp`, rounded up. With no `chert mcp` of the
  project running it is `-ms`, and the `chert` label turns red; while
  one runs the label is orange.
- `13k` is how many observations the project has, shortened as itok
  shortens its counts and rounded up: `99`, `1k`, `13k`, `1m`.
- `17.3MB` is the size of the project's stored text, in `B`, `KB`,
  `MB` or `GB` (1024-based), rounded up to a tenth.

Each value is green, amber or red against the `statusline.*` keys in
[Configuration](CONFIG.md): the reply time against `ms_green` and
`ms_amber`, the size against `size_green` and `size_amber`, and the
count by how well the cache window fits `cache.max_bytes`: amber from
`fit_amber` percent of it, red once over it. A value with nothing
measured behind it, such as the count before anything is stored, has
no color.

The badge reads only the catalog and the small heartbeat files a
running `chert mcp` keeps under the data directory's `mcp/`: it never
starts the cache or opens a partition file, and it always exits 0, so a
missing data directory or a bad `chert.toml` shows `chert:-ms,0,0B`
rather than an error.

`--color auto`, the default, colors the badge when standard input
holds Claude Code's statusline JSON or standard output is a terminal;
`--color always` and `--color never` decide for it. The project is the
JSON's `workspace.project_dir`, else its `cwd`, else the directory
`chert statusline` runs in.

To show it, set `statusLine` in `~/.claude/settings.json` to the
command, by its full path:

```json
{
  "statusLine": {
    "type": "command",
    "command": "/full/path/to/chert statusline"
  }
}
```

Or, to show it beside other badges, point `statusLine` at a script,
for example `~/.claude/hooks/chert-statusline.sh`, that hands the same
JSON to each:

```sh
#!/bin/sh
# Claude Code's statusline JSON arrives once on stdin: keep it and pass
# it to each badge.
input=$(cat)
chert=$(printf '%s' "$input" | chert statusline --color always)
itok=$(printf '%s' "$input" | itok rate --statusline)
printf '(%s) %s\n' "$chert" "$itok"
```

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/hooks/chert-statusline.sh"
  }
}
```

## Coming from cavemem

chert keeps its own data directory and never touches cavemem's files,
so the two can run side by side. To bring cavemem's history over, point
`chert import` at cavemem's database, by default `~/.cavemem/data.db`:

```sh
chert import ~/.cavemem/data.db --from cavemem
```

The database is opened read-only. Its `<private>` spans are stripped as
a hook would strip them, and each row lands in the project of its
session's working directory. Importing the same database again adds
nothing new. Then `chert install` and remove cavemem's hooks from
Claude Code, so each event is recorded once.

## Export and import

```sh
chert export memory.jsonl      # every observation, 1 JSON object per line
chert import memory.jsonl      # restore it, into this or another machine
```

Each line has the keys `body`, `id`, `ide`, `identity`, `kind`,
`project`, `ref`, `session` and `ts`. The body is the stored,
compressed text, so nothing that was stripped comes back. An import
keeps each observation's id, time, ref and project; a row already held
is skipped, and a ref that was dropped stays dropped.

### rekall

[rekall](https://github.com/pr0d1r2/rekall) mints its own candidates
from what it reads; chert never writes under `.rekall/`. The plan is for
rekall to read a `chert export --for rekall` file. That shape is not
agreed yet, so `--for rekall` exits 2 with "not built yet"; the plain
export above works today.
