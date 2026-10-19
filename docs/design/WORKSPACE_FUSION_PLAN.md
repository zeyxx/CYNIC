# Workspace Fusion Plan

Status: audit deliverable (no code mutation in this commit).
Scope: every crate under `crates/` (45 crates).
Builder: `cynic-builder` (10.10.10.119). Local cargo/rustc on cortex is forbidden.

## Method

Metrics are measured directly from the repository:

1. **LOC** — total lines in `crates/<crate>/src/**/*.rs`.
2. **Tests** — count of `#[test]` attributes under `src/`.
3. **Dependencies** — count of entries in `[dependencies]` in `crates/<crate>/Cargo.toml` (internal `cynic-*` plus external).
4. **Dependents** — number of other workspace crates whose `Cargo.toml` references this crate. A crate with zero dependents is a **leaf**.

Candidate criteria (any one): `LOC < 200`, `Tests == 0`, or `Dependents == 0`.

## Metrics (all crates)

| Crate | LOC | Tests | Deps | Dependents | Leaf | Candidate | Reason |
|---|---:|---:|---:|---:|:---:|:---:|---|
| cynic-askesis | 340 | 6 | 3 | 3 | no | no | |
| cynic-audit-engine | 4761 | 17 | 4 | 2 | no | no | |
| cynic-build | 11974 | 232 | 12 | 5 | no | no | |
| cynic-channel | 445 | 6 | 7 | 1 | no | no | |
| cynic-cli | 37838 | 126 | 50 | 0 | yes | yes | no dependents (binary root) |
| cynic-consensus | 2678 | 25 | 9 | 7 | no | no | |
| cynic-discovery | 352 | 6 | 3 | 2 | no | no | |
| cynic-discovery-host | 220 | 5 | 4 | 1 | no | no | |
| cynic-domain-cloudflare | 555 | 0 | 6 | 1 | no | yes | 0 tests |
| cynic-domain-ops | 2942 | 56 | 7 | 3 | no | no | |
| cynic-domain-porkbun | 431 | 0 | 7 | 1 | no | yes | 0 tests |
| cynic-eval | 10263 | 137 | 14 | 3 | no | no | |
| cynic-finance | 1493 | 30 | 7 | 10 | no | no | |
| cynic-fleet | 300 | 3 | 2 | 1 | no | no | |
| cynic-forensics | 1003 | 10 | 3 | 2 | no | no | |
| cynic-host | 85264 | 764 | 43 | 5 | no | no | |
| cynic-huggingface | 1527 | 0 | 3 | 2 | no | yes | 0 tests |
| cynic-inference | 14550 | 84 | 21 | 3 | no | no | |
| cynic-intel | 10633 | 182 | 15 | 5 | no | no | |
| cynic-intel-futarchy | 213 | 2 | 5 | 1 | no | no | |
| cynic-intel-soc | 594 | 11 | 6 | 2 | no | no | |
| cynic-kernel | 3539 | 35 | 43 | 3 | no | no | |
| cynic-knowledge | 5039 | 40 | 8 | 3 | no | no | |
| cynic-lifecycle | 6545 | 49 | 11 | 7 | no | no | |
| cynic-lxc | 1163 | 5 | 8 | 2 | no | no | |
| cynic-market | 1240 | 16 | 10 | 1 | no | no | |
| cynic-mcp | 777 | 13 | 7 | 0 | yes | yes | no dependents (binary) |
| cynic-network | 6596 | 107 | 20 | 8 | no | no | |
| cynic-node | 60153 | 466 | 52 | 1 | no | no | |
| cynic-node-protocol | 5878 | 93 | 7 | 5 | no | no | |
| cynic-pkg | 3714 | 30 | 6 | 2 | no | no | |
| cynic-pool | 744 | 13 | 11 | 3 | no | no | |
| cynic-protocol | 38691 | 654 | 9 | 36 | no | no | |
| cynic-proxmox | 4380 | 38 | 13 | 2 | no | no | |
| cynic-release | 1667 | 25 | 6 | 3 | no | no | |
| cynic-sandbox | 1193 | 18 | 5 | 3 | no | no | |
| cynic-secrets | 1437 | 8 | 13 | 13 | no | no | |
| cynic-solana-pay | 471 | 8 | 4 | 2 | no | no | |
| cynic-storage | 4765 | 45 | 13 | 5 | no | no | |
| cynic-surreal | 494 | 0 | 6 | 1 | no | yes | 0 tests |
| cynic-traces | 3049 | 7 | 6 | 0 | yes | yes | no dependents (orphaned library) |
| cynic-tui | 422 | 0 | 2 | 1 | no | yes | 0 tests |
| cynic-types | 850 | 12 | 4 | 16 | no | no | |
| cynic-vcs | 9575 | 164 | 7 | 4 | no | no | |
| cynic-vcs-providers | 2870 | 31 | 11 | 2 | no | no | |
| cynic-workload | 6320 | 69 | 8 | 8 | no | no | |
| cynic-zeroclaw-solana | 254 | 1 | 9 | 1 | no | no | |

## Candidate summary

- **Zero tests (5):** `cynic-domain-cloudflare`, `cynic-domain-porkbun`, `cynic-huggingface`, `cynic-surreal`, `cynic-tui`.
- **No dependents (3):** `cynic-cli` (binary root, intentional), `cynic-mcp` (binary, intentional), `cynic-traces` (orphaned library).
- **Low LOC (< 200):** none.

## Recommended fusions

### 1. Registrar domain crates → `cynic-domain-ops`

Merge `cynic-domain-cloudflare` and `cynic-domain-porkbun` into `cynic-domain-ops`.

- Rationale: both are thin provider-specific wrappers (555 and 431 LOC), share `cynic-domain-ops` + `cynic-secrets` + `cynic-protocol`, and carry zero tests.
- Owner: `cynic-domain-ops` (already the aggregate domain crate; 2942 LOC, 56 tests).
- Migration: move each provider module under `cynic-domain-ops::providers::{cloudflare,porkbun}`; re-export from the crate root to preserve call sites; update `cynic-cli` imports; delete the two crates and their workspace entries.
- Risk: low. Only `cynic-cli` consumes them.

### 2. HuggingFace adapter → `cynic-inference`

Merge `cynic-huggingface` into `cynic-inference`.

- Rationale: 1527 LOC, zero tests, consumed only by `cynic-inference` and `cynic-cli`. It is an adapter for inference, not an independent domain.
- Owner: `cynic-inference`.
- Migration: move under `cynic-inference::adapters::huggingface`; preserve public types; update `cynic-cli`.
- Risk: medium (inference is large; verify feature flags).

### 3. SurrealDB projection → `cynic-storage`

Merge `cynic-surreal` into `cynic-storage`.

- Rationale: 494 LOC, zero tests, a storage backend projection. `cynic-storage` is the sovereign storage owner (4765 LOC, 45 tests).
- Owner: `cynic-storage`.
- Migration: add a `surreal` module/feature behind the existing storage port; keep `cynic-secrets` dependency for credentials.
- Risk: medium (storage is foundational; gate with tests).

### 4. TUI kit → `cynic-cli`

Merge `cynic-tui` into `cynic-cli`.

- Rationale: 422 LOC, zero tests, consumed only by `cynic-cli`. It is a UI kit, not a reusable library.
- Owner: `cynic-cli`.
- Migration: move to `cynic-cli::tui` (or a `tui` module); update dashboard call sites.
- Risk: low.

### 5. Orphaned library `cynic-traces` — wire or remove

`cynic-traces` is a 3049 LOC domain crate with 7 tests and **no dependents**. It is documented as a first-class domain (see `docs/specs/cynic-traces-carnet.md` and `docs/foundation/DESIGN_PROTOCOL.md`), so a blind merge is not recommended.

- Option A (preferred): expose it through `cynic-cli` (a `traces` subcommand) and add an integration test, removing the orphan status.
- Option B: fold into `cynic-knowledge` as a `traces` module only if the knowledge domain accepts session-trace ownership.
- Option C: delete if the Rust rewrite is abandoned (Python `apps/cynic-data` remains the SSOT).
- Decision required before mutation.

### 6. Binary leaves — keep as-is

`cynic-cli` and `cynic-mcp` are leaves by construction (binaries). No fusion recommended. `cynic-mcp` stays a thin stdio facade over `cynic-mcp::server`.

## Watch list (not candidates by criteria, but small / single-consumer)

| Crate | LOC | Tests | Dependents | Note |
|---|---:|---:|---:|---|
| cynic-discovery-host | 220 | 5 | 1 | Could fold into `cynic-discovery` or `cynic-host`. |
| cynic-intel-futarchy | 213 | 2 | 1 | Subdomain of `cynic-intel`; candidate for folding. |
| cynic-zeroclaw-solana | 254 | 1 | 1 | Plugin; keep standalone unless consumer count stays at 1. |
| cynic-fleet | 300 | 3 | 1 | Projection helper; could fold into `cynic-host`. |
| cynic-tui | 422 | 0 | 1 | Already in fusion #4. |

## Non-goals

- This commit delivers the plan only. No crate is merged, removed, or renamed here.
- No dependency graph or public API changes.
- `infra/crate-topology.json` and `infra/registry.json` are not modified by this plan; they must be updated when fusions land.

## Build verification

Build/test gates run on `cynic-builder` (10.10.10.119), never on cortex. For a docs-only change, `make audit` (or `cynic audit all --json`) is the relevant gate; crate-level builds are unaffected.

## Commit metadata

- Branch: `agent/hermes/aegis-consortium`
- Commit identity: `hermes@agent.local`
