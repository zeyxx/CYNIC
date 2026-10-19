# Formal Proofs and Tests — Workspace Inventory

Status: baseline study (no mutation). Worktree: `agent/hermes/formal-proofs`, base `fe7d6d4b`.

## 1. Formal verification actually executed (Kani)

- Tool: `cargo kani` (Rust verification). Target Makefile: `make kani.mesh`, `make kani.layers`.
- State on cortex: `cargo kani` is **not installed**. Targets cannot run locally without `cargo kani setup`.
- 23 `#[kani::proof]` harnesses, all confined to `crates/cynic-node` and to the network low-layer surface:
  - `dns_contract/verification.rs` — 7
  - `network_mesh_apply_plan/command_plan_verification.rs` — 5
  - `network_mesh_apply_plan/policy_verification.rs` — 3
  - `network_mesh_plan/verification.rs` — 4
  - `network_root_policy/verification.rs` — 4
- Coverage themes: denial contracts, bundle file shapes, argv surfaces, profile ordering, forbidden tokens (shell-escape blocking), portability of helper paths, DNS zone stability.
- Shared test/kani gates via `#[cfg(any(test, kani))]`.

## 2. Pure-logic surface (candidate for formal verification)

- `cynic-protocol` holds the pure predicates: `decide_*` (28) and `reconcile_*` (72), plus `verify_*` (46 across workspace).
- None of these have Kani harnesses today. Highest leverage target for future proofs: pure, side-effect-free, no I/O.

## 3. Conventional unit tests

Approximate `#[test]` / `#[tokio::test]` / `#[rstest]` counts per crate (top contributors):

| Crate | Tests |
|---|---:|
| cynic-host | 845 |
| cynic-protocol | 661 |
| cynic-node | 521 |
| cynic-cli | 216 |
| cynic-intel | 195 |
| cynic-build | 167 |
| cynic-eval | 143 |
| cynic-network | 116 |
| cynic-vcs | 115 |
| cynic-inference | 106 |
| (others) | ≤95 each |

- Total across `crates/`, `apps/`, `tests/`: ~3958.
- Concentrated in host/protocol/node; long tail of lightly covered crates (discovery, market, pool, channel, lxc, huggingface).

## 4. Integration / conformance tests (`*/tests/*.rs`)

Observed families (non-exhaustive):
- Red-team / E2E: `adversarial_red_team_suite`, `multi_agent_e2e_simulation`, `trust_chain_e2e`, `autonomous_pipeline_e2e`, `storage_topology_e2e`, `forgejo_webhook_e2e`.
- Structural ratchets: ~44 `architecture_boundary.rs`, ~11 `orphan_sources_ratchet.rs`, `ownership_boundary`, `external_registry_coherence`, `layer_debt_ratchet`, `cron_topology_coherence`.
- Proof-transport surfaces: `peer_proof_transport`, `runtime_proof_http_fetcher_security`, `mesh_plan_registry_proof`.
- Build/workflow: `build_workflow_e2e`, `workload_build_integration`, `installed_runtime`, `build_proof_log_roundtrip`.

## 5. Property-based testing

- No `proptest` / `quickcheck` / `arbitrary` framework dependency found.
- Only ad-hoc randomized-ish assertions inside modules; no systematic property tests.

## 6. Formal specification (aspirational)

- `docs/foundation/whitepaper/06_SPECIFICATION_FORMELLE_LEAN4_ET_DAG.md` declares Lean4 theorems (`no_execution_without_admit`, `authority_flows_downward_only`, `state_mutation_soundness`) but they still carry `sorry`.
- No `.lean` files anywhere in the repo. Treat as intent, not proof.

## 7. Runtime proof domain (not formal verification)

- `cynic-protocol::runtime::proofs`: `RuntimeProof`, `RuntimeProofBundle`, `RuntimeProofPersistenceRecord`, `InferenceStatusProof`, `HostProofSubmission`.
- Adapters/ports in `cynic-node` (`runtime_proof_http_fetcher`, `kernel_source_repository_proof_commit`) and `cynic-host` (`runtime_proof`, `kernel_proof_persistence`).
- Gates: `make check` (`cynic audit crate --json`), `make audit` (`cynic audit all --json`), `audit.deployed`.

## 8. Observed gaps

1. Kani is scoped to network modules only; pure protocol predicates (decide/reconcile) are unverified.
2. No property-based testing anywhere.
3. Lean4 specification is documentation only (`sorry`).
4. Test coverage is uneven across crates.
5. `cargo kani` is unavailable on the build host, so Kani targets are currently non-runnable locally.

## 9. Candidate next steps (not yet executed)

- Add Kani harnesses for pure predicates in `cynic-protocol` (highest leverage, lowest risk).
- Introduce `proptest` for `decide_*` / `reconcile_*` predicates.
- Either implement Lean4 or mark the spec explicitly aspirational.
- Run a coverage ratchet across crates (orphan sources ratchet style).

No code was changed by this study.
