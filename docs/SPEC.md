# SPEC

## §G GOAL

human pages, prose (caveman only in `SPEC.md`): `README.md` routes to `docs/BENCHMARKS.md`, `docs/INTEGRATION.md` (install per IDE, rekall export), `docs/CONFIG.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `docs/LLM-DISCLAIMER.md`, `THIRD-PARTY-NOTICES.md`.

## §N NAV

rel|path|lens
up|.|-
self|docs|human pages: README, BENCHMARKS, INTEGRATION, CONTRIBUTING, SECURITY, LLM-DISCLAIMER
sib|src|crate `chertmem`: lib + bin `chert`, 1 node per module
sib|benches|benchmarks: fixtures, suites, baseline, regression gate
sib|dev|root `flake.nix` devShell & `dev` crate (`publish = false`) gate steps

## §C CONSTRAINTS

- page = claim first, evidence second, how-to last; readable on phone.
- ∀ number dated w/ recount method (`paradigm/V4`); counts generated where a generator exists (`paradigm/V34`).
- `docs/BENCHMARKS.md` (D29): how to run, how to read output, targets & why (`.:§C`), fixture sizes, machine & date ∀ measured number, how baseline is updated, regression rule.
- `docs/CONFIG.md`: `inject.on_prompt` carries speed disclaimer w/ measured cost from `docs/BENCHMARKS.md`.

## §I INTERFACES

- page: `README.md`, `docs/BENCHMARKS.md`, `docs/INTEGRATION.md`, `docs/CONFIG.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `docs/LLM-DISCLAIMER.md`, `THIRD-PARTY-NOTICES.md`.

## §V INVARIANTS

V1: ∀ number in `docs/BENCHMARKS.md` ≡ a row in `benches/baseline.toml` (drift test).
V2: `docs/CONFIG.md` key list ≡ `src/config` keys (parity test).
V3: ⊥ private repo named (`.:V12`).

## §T TASKS

id|status|task|cites
T1|.|`README.md` + routing|-
T2|.|`docs/BENCHMARKS.md` from first baseline (`.:T16`)|V1
T3|.|`docs/CONFIG.md` w/ parity test|V2
T4|.|`docs/INTEGRATION.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `docs/LLM-DISCLAIMER.md`, `THIRD-PARTY-NOTICES.md`|V3

## §B BUGS

id|date|cause|fix
