# Built by an LLM, deliberately and in the open

This repository (code, spec, tests and prose) was written mostly by
[Claude Code](https://claude.com/claude-code) running Anthropic's
**Claude Opus 5.5**, under the [paradigm](https://github.com/pr0d1r2/paradigm)
lifecycle. Every commit carries a `Co-Authored-By: Claude Opus 5.5`
trailer; compare `git log --format=%B | grep -c 'Co-Authored-By: Claude Opus 5.5'`
with `git rev-list --count HEAD` to check. The human named in
[`LICENSE`](../LICENSE) owns the project, makes its decisions and is
accountable for what ships.

## Why say it at all

An LLM writes plausible code, and plausible is not correct. A reader who
does not know how a repository was produced cannot calibrate how hard to
look at it. chert holds the memory of your agent sessions and promises
that a cached answer equals the disk's and that private text never
reaches the store, so the standard here is: **a claim in prose is a
defect until something executes it.**

## How it was built

- **Intent first.** [`SPEC-PROMPT.md`](../SPEC-PROMPT.md) holds the
  owner's intent and numbered, dated decision rounds, frozen once the
  spec was written. In round 2 the owner told the agent to accept its
  own proposals for most open questions, so those answers are the
  agent's, standing until the owner amends them; the owner's review of
  them is still an open task (root `SPEC.md`, T15).
- **The spec is the law.** [`SPEC.md`](../SPEC.md) and one `SPEC.md` per
  node below it carry `V` invariants (what must stay true), `T` tasks,
  `R` research (the measurements constraints rest on) and `B` bugs,
  each bug paired with the invariant that now catches it.
- **Waves of agents.** The build ran in waves: a lead agent split the
  work by node, parallel worker agents each built one node in its own
  git worktree, test first (a failing RED commit, then the GREEN one),
  and the lead merged what passed the gate. Outward acts (pushing,
  publishing, opening issues in other repositories) wait for the owner's
  approval.
- **Guardrails, not watching.** Entering the dev shell points git at
  hooks that run [hk](https://github.com/jdx/hk) against one gate,
  declared row by row in [`.pklith`](../.pklith). Each check landed in
  its own commit with a planted violation it refused. The gate enforces,
  among others: clippy pedantic as errors, `unsafe` forbidden, a test
  commit before any commit adding Rust code, every Rust module wired to
  a test file, a line coverage floor that may only rise, commit message
  shape, and no local paths or private names in committed text.

## Bugs found after green

Each of these was found after the spec or code it concerns had been
accepted, and each now has an invariant that would have caught it. The
root bugs were found in a drift review of the spec before their code
was written; the fix commits are this repository's own.

| Bug | What went wrong | Caught now by | Fix |
| --- | --- | --- | --- |
| root B1 | A ref was the shortest unique ULID prefix at read time; rows within about a second share 8 characters, so a later row could make an issued ref ambiguous | root V15: refs issued once, stored, matched exactly | `a9ab8fe` spec, `2de4a17` code |
| root B2 | Writes were deduplicated by payload content, so two real identical events (the same command twice) collapsed into one | root V16: dedupe only by an event identity the IDE supplies | `65be014` spec, `707a094` code |
| root B3 | Every store open replayed the spool, hooks included, so a backlog pushed hooks past their 5 ms target | `src/store` V10: hooks never replay; reading opens do | `0cf8d5b` spec, `a1a7007` code |
| `src/store` B1 | A catalog rebuild restarted the generation counter, so a running cache could take changed data as fresh | `src/store` V11: the generation never repeats | `1b03e59` |
| `src/store` B2 | A deleted catalog lost the dropped markers, so a dropped ref could be issued again | `src/store` V12: drop tombstones outside the catalog | `07a0ddf` |
| `src/search` B1 | The query was tokenized raw while stored bodies are compressed, so plain words missed compressed rows | `src/search` V5: queries go through the same grammar | `569793b` |
| `src/grammar` B1 | Ported faithfully from cavemem 0.2.1: the article filter cut a leading `a` or `the` off a path (`a/b.ipynb` became `/b.ipynb`) | `src/grammar` V2: paths survive | `77d0776` |
| `src/install` B1 | Following cavemem, the MCP server was written into `settings.json`, where Claude Code never reads MCP servers | `src/install` V4: the server is registered where Claude Code reads it | `c5be707` |
| `dev` B1 | The workspace stub had no behaviour for a test to pin, so its commit could not satisfy the test-first rule | `dev` V8: an exempt list, each entry with a bug row | `e236b64` |
| `dev` B2 | A gate step failed without output: a parallel cargo step relinked the binary it was running | `dev` V9, later V10 | `b376c7e` |
| `dev` B3 | After B2's fix, every `cargo run` still relinked its binary, so parallel steps raced | `dev` V10: such steps run alone | `4fc86ad` |
| `dev` B4 | Over-long and miscited commit subjects landed, found only by hand | `dev` V11: a commit-msg guard | `1c774ac` |
| `dev` B5 | The coverage floor, set at the exact measurement, failed a worker's checkout of the same code by 0.02 points; the cause is not found yet | `dev` V13: the floor leaves 0.10 below the measurement | `bb788c5` |

## What a reader should check

1. **Does the gate run for you?** `nix develop`, then `hk check --all`.
2. **Does the bug table look like a real log or a curated one?** Compare
   it with the `B` tables in the specs.
3. **Do the invariants have runners?** An invariant that nothing
   executes is exactly the defect this project exists to avoid.
4. **Do the privacy promises hold?** Put a `<private>` span in a prompt,
   then look for it with `chert export`.

## Accountability

The human named in [`LICENSE`](../LICENSE) is responsible for this
code, including the parts a model wrote and the parts nobody caught.
"The LLM wrote it" explains provenance; it never transfers
responsibility.
