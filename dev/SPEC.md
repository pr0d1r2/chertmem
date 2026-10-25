# SPEC

## §G GOAL

root `flake.nix` devShell w/ pinned toolchain & gate tools, laid out as public Rust member lablith (pklith `lib.devShell`, tools pinned by tag, ∀ input follows `nixpkgs-lock`); `dev/` = crate `chertmem-dev` (`publish = false`) for gate steps in Rust (owner 2026-10-24): `name-guard`, `no-core-worktree`, `lock-graph`, `rust-mirror`, `tdd-order`, `commit-msg`, `bench-gate`, target/suite parity.

## §N NAV

rel|path|lens
up|.|-
self|dev|root `flake.nix` devShell & `dev` crate (`publish = false`) gate steps
sib|src|crate `chertmem`: lib + bin `chert`, 1 node per module
sib|benches|benchmarks: fixtures, suites, baseline, regression gate
sib|docs|human pages: README, BENCHMARKS, INTEGRATION, CONTRIBUTING, SECURITY, LLM-DISCLAIMER

## §C CONSTRAINTS

- nix flake + direnv per `paradigm/V20`; nixpkgs follows nixpkgs-lock; shell ∀ `paradigm/V28` systems.
- tools on PATH: rust toolchain, hk, pkl, `mth`, `sherd` (≥ 0.5.4), `itok`, cargo-deny, cargo-nextest, Node (grammar oracle only). plus ∀ tool `pklith gen` names in `nix/pklith.nix`, listed in the flake too ∴ `pklith lay` finds each before its step exists.
- human reload checkpoint after shell lands (`.:T6`).

## §I INTERFACES

- cmd: `nix develop`; `.envrc` = `use flake`.
- cmd: `cargo run -q -p chertmem-dev -- <guard> [files...]`; guard ∈ `name-guard` `no-core-worktree` `lock-graph` `rust-mirror` `tdd-order` `commit-msg <file>` `bench-gate` `perf-doc [--check]`.
- file: `dev/tdd-order.exempt` — full commit id + reason per line; ∀ entry has a §B row. exit 0 hold · 1 finding (file:line & fix on stderr) · 2 usage | unreadable input.
- file: `.private/denylist` — untracked, 1 name per line, `#` comments (`paradigm/V37`).
- file: `dev/public-repos.toml` — `self = "owner/name"` & `[[repo]]` rows `name`, `public` (YYYY-MM-DD) (V14).

## §V INVARIANTS

V1: ∀ hk step's tool on dev shell PATH.
V2: Node ∈ dev shell only, ⊥ in `chert` closure (`.:§C`).
V3: `name-guard`: ∀ given file ∋ denylist name (case-insensitive) → 1 finding naming file & line, ⊥ echoing the name into committed output; ⊥ denylist → hold; allowlist half = V14 (`.:V12`).
V4: `no-core-worktree`: `core.worktree` set in shared `.git/config` → finding (`paradigm/V15`).
V5: `lock-graph`: `flake.lock` holds exactly 1 nixpkgs node & ∀ input named `nixpkgs` | `nixpkgs-lock` follows root `nixpkgs-lock` → hold; else finding naming node.
V6: ∀ guard proven by unit test rejecting planted violation & passing clean twin (`paradigm/V21`).
V7: `rust-mirror` (port of xenolith v0.1.0 rule, as lablith copies it): ∀ tracked `.rs` w/ a bodied fn has wired mirror (`<m>.rs` ↔ `<m>/tests.rs`, `lib.rs`|`mod.rs` ↔ `tests.rs`, `#[cfg(test)] mod tests;`); ∀ `tests.rs` has its module; `main.rs` = shim w/ only `fn main`; `build.rs`, `vendor/`, crate `tests/` exempt.
V8: `tdd-order` (same port, Rust rule only, ⊥ shell scripts here): ∀ commit in range adding a `.rs` outside `tests/` has a `test:` | `test(` commit before it in range, unless its full id ∈ `dev/tdd-order.exempt`; missing | malformed exempt file → finding.
V9: ∀ gate step running `cargo run` depends on step `build` (`cargo build -q --workspace --bins`, after clippy); `test` waits for ∀ such step ∴ ⊥ step relinks a binary another is executing (B2). [superseded by V10]
V10: ∀ gate step running `cargo run` | `cargo build` is hk `exclusive` ∵ cargo re-links `target/debug/<bin>` (new inode) on every run & build, even w/o rebuild ∴ 2 such steps at once race; `test` waits for them (B3).
V11: `commit-msg`: subject `type(scope)?: claim`, type ∈ feat fix docs test chore refactor perf build ci, ≤ 72 chars; ≥ 1 body line before trailers; final paragraph carries `Paradigm-Stage: <stage>` from `paradigm` §I vocabulary (`paradigm/V36`, `paradigm/V43`); git's own Merge, Revert, fixup!, squash! subjects pass. `name-guard` reads the message too (B4).
V12: `commit-msg` reads strictly (wave 1, as built): ∀ line of final paragraph = `Key: value` (key letters, digits, `-`; indented continuation ok) ∴ trailer beside prose = finding; trailer right under subject w/o blank = body; stage `<n>` positive, ⊥ leading zero; key match case-sensitive `Paradigm-Stage:`; scope chars lowercase, digits, `/ . _ -`; Merge, Revert, fixup!, squash! subjects skip ∀ rule.
V13: `.coverage` floor = `sherd coverage --record` result − 0.10, rounded down to 0.01 ∴ a worker's worktree, which measured 0.02 lower than the main tree at one commit, passes on equal code; spread > 0.10 = a finding to chase. ratchet still only rises (`paradigm/V25`) (B5).
V14: `name-guard` always, denylist or not: ∀ repo named | linked in given file (after `github.com/` | `github:`, ∀ `pr0d1r2/<name>`) ∈ `dev/public-repos.toml` → hold; else 1 finding naming file, line & repo (public name, ⊥ leak). row = `name` + `public` = date `gh repo view` showed PUBLIC; `self` = this repo, exempt (pending first push, `.:T13`); missing | malformed list → exit 2. adding row = own reviewed commit (`paradigm/V37`).

## §T TASKS

id|status|task|cites
T1|x|root `flake.nix` devShell + `.envrc`|V1,V2
T2|x|`dev` crate w/ `bench-gate`: `bench-record` + gate on push, 1k & 10k-p52 (`benches:R5`)|V6
T3|x|`name-guard`, RED → GREEN → gate row|V3,V6
T4|x|`no-core-worktree`, RED → GREEN → gate row|V4,V6
T5|x|`lock-graph`, RED → GREEN → gate row|V5,V6
T6|x|`rust-mirror`, RED → GREEN → gate row|V7,V6
T7|x|`tdd-order` + exempt file, RED → GREEN → gate row|V8,V6
T8|x|`commit-msg` guard + hook, RED → GREEN → gate|V11,V6
T9|x|`perf-doc [--check]`: `docs/PERFORMANCE.md` from `benches/baseline.toml` & root §C targets, RED → GREEN → gate row at commit|V6
T10|.|`name-guard` allowlist half + `dev/public-repos.toml` seeded from ∀ repo committed text names, `gh repo view` 2026-10-25, RED → GREEN|V14,V6

## §B BUGS

id|date|cause|fix
B1|2026-10-24|workspace stub (`src:T1`) laid by hand w/ bodiless `lib.rs` & `main.rs`: ⊥ behaviour for a RED test to pin ∴ exempt in `dev/tdd-order.exempt`|V8
B2|2026-10-24|`name-guard` failed w/o denylist & w/o output in pre-commit while clippy, rustdoc & doctest rebuilt `chertmem-dev` beside it: a cargo step relinked the binary another was executing (lablith & actlith B9)|V9
B3|2026-10-24|after B2's fix `no-core-worktree` failed "could not execute process `target/debug/chertmem-dev` (never executed)": measured, ∀ `cargo run` & `cargo build` replaces the binary's hard link (inode 12182331 → 12182334 → 12182340 → 12182342) even when up to date ∴ parallel `cargo run` steps race each other, ⊥ only the build|V10
B4|2026-10-24|16 subjects over 72 chars & 1 citing wrong id landed in guardrails stage, found only by hand twice: ⊥ step read commit messages|V11
B5|2026-10-24|wave 6: at ec3324d the main tree measured 98.22%, a worker worktree 98.20%, so the floor set at the exact measurement failed that worker's pre-push on equal code; cause ⊥ found yet (checkout path in tests?)|V13
