# SPEC

## §G GOAL

`dev/`: dev shell flake w/ pinned toolchain & gate tools; `dev` crate (`publish = false`) for gate steps: `bench-gate`, target/suite parity, name guard input.

## §N NAV

rel|path|lens
up|.|-
self|dev|dev shell flake & `dev` crate (`publish = false`) gate steps
sib|src|crate `chertmem`: lib + bin `chert`, 1 node per module
sib|benches|benchmarks: fixtures, suites, baseline, regression gate

## §C CONSTRAINTS

- nix flake + direnv per `paradigm/V20`; nixpkgs follows nixpkgs-lock; shell ∀ `paradigm/V28` systems.
- tools on PATH: rust toolchain, hk, pkl, `mth`, `sherd` (≥ 0.5.4), `itok`, cargo-deny, cargo-nextest, Node (grammar oracle only).
- human reload checkpoint after shell lands (`.:T6`).

## §I INTERFACES

- cmd: `nix develop ./dev`; `.envrc`.
- cmd: `cargo run -p dev -- bench-gate`.

## §V INVARIANTS

V1: ∀ hk step's tool on dev shell PATH.
V2: Node ∈ dev shell only, ⊥ in `chert` closure (`.:§C`).

## §T TASKS

id|status|task|cites
T1|.|`dev/flake.nix` + `.envrc`|V1,V2
T2|.|`dev` crate w/ `bench-gate`|-

## §B BUGS

id|date|cause|fix
