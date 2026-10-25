# Security policy

## Reporting a vulnerability

Please report vulnerabilities privately, through GitHub's
[private vulnerability reporting](https://github.com/pr0d1r2/chertmem/security/advisories/new),
not in a public issue. Include what you ran, what happened, and what you
expected.

## Supported versions

chertmem has no release yet. Fixes land on `main`; once releases exist,
the latest one is supported.

## What chert stores, and where

chert is a memory of your agent sessions, so its store holds what those
sessions saw. Treat its data directory like your shell history, or more
carefully:

- **What**: every prompt you send; every tool call's name, input and
  output (file contents an agent read, command output, error text); turn
  and session summaries taken from the agent's own messages; and each
  session's id, IDE and working directory.
- **Where**: the data directory, `~/.local/share/chert` unless
  `storage.data_dir` says otherwise: `catalog.db`, one SQLite file per
  partition under `chunks/`, cold partitions as `.zst` files, and
  `spool/`, which holds events waiting to be stored and, under
  `spool/unreadable/`, raw payloads a hook could not parse (their
`<private>` spans already stripped). Also there:
  chert's own search and mining indexes. `chert export` writes the same
  observations to the file you name.
- **In memory**: `chert mcp` keeps the current project's recent window
  (`cache.window`) in memory while it runs.
- **In your IDE's config**: `chert install` writes hook commands to
  `~/.claude/settings.json` and the MCP server entry to `~/.claude.json`,
  each with a `.chert.bak` backup of the file as it was.

Stored text is compressed, not encrypted. Anyone who can read the data
directory can read your sessions.

## What chert promises

- **Private spans are stripped before any write.** Text between
  `<private>` and `</private>` is removed from every payload before it
  is compressed, stored, spooled, cached or exported; an unclosed
  `<private>` drops everything after it. A tool call naming a path in
  `privacy.exclude` is not stored at all. Neither reaches the store,
  the cache or an export.
- **No network by default.** chert makes no network connection unless
  you set `embedding.provider = "ollama"`, and then only to the Ollama
  server `OLLAMA_HOST` names, over plain HTTP; an `https://` host is
  refused. No other setting or flag sends anything anywhere.
- **No server and no daemon.** Hooks are short processes; `chert mcp`
  runs only while your IDE runs it, over standard input and output.
- **It edits only what it names.** Outside its data directory, chert
  writes only the two IDE files above, and `chert uninstall` removes
  only its own entries. It reads a cavemem database read-only, never
  writes `.rekall/`, and `chert mine` only reports.

In scope: a byte of a `<private>` span or an excluded path reaching the
store, the spool, the cache or an export; any network connection other
than the one `embedding.provider` asks for; a write outside the data
directory and the two IDE files; a hook payload that makes chert run
something.
