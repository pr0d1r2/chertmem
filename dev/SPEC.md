# SPEC

## §G GOAL

root `flake.nix` devShell w/ pinned toolchain & gate tools, laid out as public Rust member lablith (pklith `lib.devShell`, tools pinned by tag, ∀ input follows `nixpkgs-lock`); `dev/` = `dev` crate (`publish = false`) for gate steps: `bench-gate`, target/suite parity, name guard input.

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
- cmd: `cargo run -p dev -- bench-gate`.

## §V INVARIANTS

V1: ∀ hk step's tool on dev shell PATH.
V2: Node ∈ dev shell only, ⊥ in `chert` closure (`.:§C`).

## §T TASKS

id|status|task|cites
T1|x|root `flake.nix` devShell + `.envrc`|V1,V2
T2|.|`dev` crate w/ `bench-gate`|-

## §B BUGS

id|date|cause|fix
