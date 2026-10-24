# SPEC.md FORMAT -- EXTENSIONS

Cavekit's `FORMAT.md` fixes seven sections. This document is the DELTA a
project may add on top of it, and nothing here needs a particular tool to be
true: a spec that follows it is still a cavekit spec, read by hand or by
whatever you already run.

The vendored `FORMAT.md` beside this file is upstream's, unmodified. See
`.format-upstream` for the exact revision it matches. This file never edits
it -- it says what is ADDED, so the difference between the two is a thing a
reader can find rather than a thing they have to notice.

**Every extension here is OPTIONAL.** A spec carrying none of them is
unchanged, and must stay that way: an extension that makes existing specs
fail is a fork, whatever it is called.

Each one carries what was MEASURED before it was claimed, because a letter
is only free until somebody else spends it, and a proposal without a
denominator is an opinion. The sweep behind those numbers ran on
**2026-08-28** over **256 distinct specs in 62 projects**; it is a private
corpus, so the counts travel and the names do not. Claiming both letters
moved that corpus from 114 clean specs to 114 -- the whole cost fell on one
file that was already failing other rules.

Run the sweep today and it reports 107, not 114. Both are right: the 114 was
taken before `V42` existed, and that rule ships in the same release and
accounts for the other seven. The condition is stated rather than the number
quietly swapped, because a figure a reader cannot reproduce is worse than one
that is merely old.

An earlier version of this file said 619 specs. That number counted
snapshots and checkouts as separate projects, and was wrong by more than
half; the per-letter counts below survived the correction unchanged, but
the denominator around them did not. It is stated this plainly because a
measurement nobody can check is worth exactly as much as the care taken
over it.

## SECTION ORDER

Fixed order, fixed headers, addressable -- the same rule upstream states. A
section may be absent, but is never reordered.

| # | section | defined by |
|---|---------|------------|
| 1 | `## §G GOAL` | cavekit `FORMAT.md` |
| 2 | `## §D DESCRIPTION` | **this document** |
| 3 | `## §F FEDERATION` | **this document** |
| 4 | `## §N NAV` | **this document** |
| 5 | `## §C CONSTRAINTS` | cavekit `FORMAT.md` |
| 6 | `## §I INTERFACES` | cavekit `FORMAT.md` |
| 7 | `## §R RESEARCH` | cavekit `FORMAT.md` |
| 8 | `## §V INVARIANTS` | cavekit `FORMAT.md` |
| 9 | `## §T TASKS` | cavekit `FORMAT.md` |
| 10 | `## §B BUGS` | cavekit `FORMAT.md` |

## THE SECTIONS

### `## §D DESCRIPTION`

The header must carry **`description`** -- matched as a stem, case-insensitively, with qualifiers free to follow. `## §D DESCRIPTION`, `## §D Description` and `## §D — Description` all name it.

A cold-reader description of what this project IS: its identity, scope, and the kind of work or system a reader should expect to find here. This is not the GOAL -- `§G` says what the code must DO -- but the project-level context that lets a reader understand the rest of the spec before following its constraints and interfaces.

```
## §D DESCRIPTION
## §D DESCRIPTION
A CPU-only Rust library and CLI for reading and enforcing cavekit SPEC files.
```

*Measured:* 73 of 256 distinct specs, across 6 projects, carry `§D`; all 73 are headed `— description`. No spec spells `§D` as another concept, so the corpus records 0 collisions. The denominator is the corrected distinct-spec/project population from B27·B28.

### `## §F FEDERATION`

The header must carry **`federation`** -- matched as a stem, case-insensitively, with qualifiers free to follow. `## §F FEDERATION`, `## §F Federation` and `## §F — Federation` all name it.

The edges this directory DECLARES. A spec federated over a directory tree is one file per directory, so each one names the tree it belongs to rather than restating it. Bullets, no ids -- an edge is a fact about the file, not an addressable item -- a PIPE TABLE, `dir|owns|⊥owns|tokens`, one row per child directory. What a node does NOT own is carried beside what it does, because an edge that names only the first half sends a reader looking in the wrong node. A citation crossing an edge is written in BACKTICKS: a bare id is read against THIS spec, so an unquoted one resolves to the wrong rule or to none.

```
## §F FEDERATION
dir|owns|⊥owns|tokens
worker|the queue consumer and its retries|the schema, which is `store`'s|-
store|the schema and every migration|anything reading the queue|-
```

*Measured:* 2 specs, one project, head `§F FEDERATION`. One OTHER project heads `§F` as feature flags -- the single collision claiming this letter creates, named here rather than discovered later: that section holds feature flags, so no header rewrite repairs it and the content has to move.

### `## §N NAV`

The header must carry **`nav`** -- matched as a stem, case-insensitively, with qualifiers free to follow. `## §N NAV`, `## §N Nav` and `## §N — Nav` all name it.

The DERIVED half: what a reader follows to reach a neighbour. Written down rather than computed at read time, so a spec read on its own still says where it sits. Independent of the section above -- a leaf declares no edges and still has neighbours -- so neither implies the other.

```
## §N NAV
rel|path|lens
up|.|-
self|src|the format rules and the verbs
sib|tests|fixtures and the corpus
```

*Measured:* 19 specs -- one project, federated over a tree -- head `§N NAV`. No other project spells `§N` at all, so the letter is unclaimed and claiming it collides with nothing. One project is one adopter, however many files it spans; the count is specs because the SECTION is what is being proposed.

## THE MARKERS

A marker is an extension that is not a new section but a note ON a line. It
is addressed by its words rather than by a letter, so it needs a canonical
spelling for the same reason a section does: two projects writing one thing
two ways is one thing nothing can find twice.

Markers are written in square brackets at the end of the statement they mark,
and inside backticks they are literal -- an example of a marker is not a
marker, exactly as an example of a citation is not a citation.

### `[superseded by ...]`

Written on a `§V` statement. A rule that has been REPLACED, marked rather than deleted. Deleting it would free the id for reuse and strand every citation that still names it, so the statement stays and the mark says it is no longer in force. More than one replacement may be named, because a rule that is SPLIT is replaced by several. The rules named must be live: pointing at a statement that is itself superseded sends a reader to law that is also dead, so the chain is written to its live end.

```
V3: **the old rule.** [superseded by V9]
V4: **a rule that was split.** [superseded by V10, V11]
```

*Measured:* ONE other project writes this bracket form, across five revisions of its spec -- and it is the project this marker was taken from, so it is a source rather than independent corroboration. No unrelated project uses it. One further project retires rules in PROSE -- `~~V11: ...~~ superseded by V17, V18` -- which is where the wording comes from, and why more than one replacement is allowed. That prose stays legal: nothing here turns an existing spec red.

## ADDRESSING

`§<S>.<n>` addresses item `n` of section `S`, exactly as upstream
defines it -- `§F.2` is the second edge declared.

This is the whole reason a letter must mean ONE thing everywhere. A citation
resolves against the READER's section set, not the writer's, so two projects
spelling one letter for two concepts break every citation that travels
between them -- silently, because both files still parse.

## ADOPTING THESE

Copy this file next to your `FORMAT.md` and write the sections. Nothing else
is required: the letters are ordinary cavekit sections that upstream has no
opinion about yet, and a spec that uses them is legal to every reader that
tolerates an unknown letter -- which upstream's own rule requires.

If upstream adopts a letter, it stops being an extension and leaves this
document. That is the intended end state, not a defeat: this file exists to
record what one copy leads with, and a shorter one means the lead was taken.

## THIS FILE IS GENERATED

Rendered from the same constants the checker reads, and frozen by a test --
so it cannot describe a format the tool does not enforce. Regenerate with
`mth extensions`; never edit it by hand.
