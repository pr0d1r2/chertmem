# SPEC

## §G GOAL

root `flake.nix` devShell w/ pinned toolchain & gate tools, laid out as public Rust member lablith (pklith `lib.devShell`, tools pinned by tag, ∀ input follows `nixpkgs-lock`); `dev/` = crate `chertmem-dev` (`publish = false`) for gate steps in Rust (owner 2026-10-24): `name-guard`, `no-core-worktree`, `lock-graph`, `rust-mirror`, `tdd-order`, `bench-gate`, target/suite parity.

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
- cmd: `cargo run -q -p chertmem-dev -- <guard> [files...]`; guard ∈ `name-guard` `no-core-worktree` `lock-graph` `rust-mirror` `tdd-order` `bench-gate`.
- file: `dev/tdd-order.exempt` — full commit id + reason per line; ∀ entry has a §B row. exit 0 hold · 1 finding (file:line & fix on stderr) · 2 usage | unreadable input.
- file: `.private/denylist` — untracked, 1 name per line, `#` comments (`paradigm/V37`).

## §V INVARIANTS

V1: ∀ hk step's tool on dev shell PATH.
V2: Node ∈ dev shell only, ⊥ in `chert` closure (`.:§C`).
V3: `name-guard`: ∀ given file ∋ denylist name (case-insensitive) → 1 finding naming file & line, ⊥ echoing the name into committed output; ⊥ denylist → hold (CI half = allowlist, later) (`.:V12`).
V4: `no-core-worktree`: `core.worktree` set in shared `.git/config` → finding (`paradigm/V15`).
V5: `lock-graph`: `flake.lock` holds exactly 1 nixpkgs node & ∀ input named `nixpkgs` | `nixpkgs-lock` follows root `nixpkgs-lock` → hold; else finding naming node.
V6: ∀ guard proven by unit test rejecting planted violation & passing clean twin (`paradigm/V21`).
V7: `rust-mirror` (port of xenolith v0.1.0 rule, as lablith copies it): ∀ tracked `.rs` w/ a bodied fn has wired mirror (`<m>.rs` ↔ `<m>/tests.rs`, `lib.rs`|`mod.rs` ↔ `tests.rs`, `#[cfg(test)] mod tests;`); ∀ `tests.rs` has its module; `main.rs` = shim w/ only `fn main`; `build.rs`, `vendor/`, crate `tests/` exempt.
V8: `tdd-order` (same port, Rust rule only, ⊥ shell scripts here): ∀ commit in range adding a `.rs` outside `tests/` has a `test:` | `test(` commit before it in range, unless its full id ∈ `dev/tdd-order.exempt`; missing | malformed exempt file → finding.
V9: ∀ gate step running `cargo run` depends on step `build` (`cargo build -q --workspace --bins`, after clippy); `test` waits for ∀ such step ∴ ⊥ step relinks a binary another is executing (B2).

## §T TASKS

id|status|task|cites
T1|x|root `flake.nix` devShell + `.envrc`|V1,V2
T2|.|`dev` crate w/ `bench-gate`|-
T3|x|`name-guard`, RED → GREEN → gate row|V3,V6
T4|x|`no-core-worktree`, RED → GREEN → gate row|V4,V6
T5|x|`lock-graph`, RED → GREEN → gate row|V5,V6
T6|.|`rust-mirror`, RED → GREEN → gate row|V7,V6
T7|.|`tdd-order` + exempt file, RED → GREEN → gate row|V8,V6

## §B BUGS

id|date|cause|fix
B1|2026-10-24|workspace stub (`src:T1`) laid by hand w/ bodiless `lib.rs` & `main.rs`: ⊥ behaviour for a RED test to pin ∴ exempt in `dev/tdd-order.exempt`|V8
B2|2026-10-24|`name-guard` failed w/o denylist & w/o output in pre-commit while clippy, rustdoc & doctest rebuilt `chertmem-dev` beside it: a cargo step relinked the binary another was executing (lablith & actlith B9)|V9
