# Agent Isolation & Experimentation — Specification

- Schema: `cynic.agent_isolation_experimentation.spec.v1`
- Status: design proposal. No fleet, network, secret, or service mutation is authorized by this document.
- Authority: observed runtime state → `contracts/foundation.json` → `infra/runtime-authority.json` → `infra/registry.json` → workspace `Cargo.toml` → canonical crates.
- Crate policy: `contracts/pruning-policy.json` (ratified 2026-09-28). A crate is created only under `crate_justified`; a module is enough otherwise. `suspect_over_splitting` applies to any new crate.
- Extends: `docs/design/AGENT_FLEET_HOSTING_DESIGN.md` (phases F0–F5), `docs/specs/agent-degradation-and-execution-authority.md` (coordination degradation + execution authority).

Language invariant: all identifiers, schema names, error codes and Rust symbols in this document are English. Prose follows the existing `docs/design` corpus.

## 1. Purpose

The workstation (Cortex) is not the durable host for agent sessions. A dedicated Proxmox VM (`cynic-agent-host`) is the primary isolation boundary; the permanent Builder LXC is a secondary worker. This specification formalizes three things that are currently either partial or absent:

1. **Graceful degradation of the agent host** — what an agent may do when the hypervisor, the VM, or the Builder is unreachable.
2. **A data-driven placement policy for the Builder** — permanent vs ephemeral, decided on observed evidence, not on a fixed assumption.
3. **An experimentation protocol** — how an ephemeral VM/container is provisioned, observed, evolved and destroyed with read-back proof.
4. **De-bloating `cynic-host`** — moving capabilities it should not own into their existing owning crates.

## 2. Observed state (what is real, verified in-tree)

### 2.1 Functional (implemented, with tests)

| Surface | Location | State |
|---|---|---|
| Host/sandbox contracts | `crates/cynic-protocol/src/agent_host_sandbox.rs` | `AgentHostDesired`, `AgentSandboxAllocation`, `AgentSandboxObservation`, `AgentSandboxReceipt`, `NotificationOutboxEntry` + fail-closed validators + tests |
| Isolation profiles | `crates/cynic-protocol/src/agent_isolation.rs` | `SandboxStrategy::{BwrapEphemeral,LxcUnprivileged}`, `NetworkIsolationProfile`, `AgentIsolationProfile`, `CapabilityEgressRoute` |
| Coordination degradation | `crates/cynic-protocol/src/agent_coordination_continuity.rs` | `RuntimeFenced` / `DegradedOperatorNotification` / `BlockedUnknown`, `reconcile_agent_coordination_continuity`, tests |
| Placement (kernel/build) | `crates/cynic-protocol/src/runtime_placement.rs` | `RuntimePlacementDesiredState`, `KernelPlacement`, `BuildRunnerPlacement`, fleet-inventory read-back |
| Proxmox adapters | `crates/cynic-proxmox/src/adapters/` | `proxmox_clone`, `proxmox_control`, `proxmox_disk_extension`, `proxmox_substrate`, `ed25519_credential_lease_verifier` |
| Agent-host observation | `crates/cynic-host/src/agent_host/observe.rs` | `ProxmoxRestAgentHostObserver`, `ProxmoxSshAgentHostObserver`; dynamic node resolution, TLS fail-closed, tests |
| Agent-host plan | `crates/cynic-host/src/agent_host/decision.rs` | `plan_agent_host_vm` → 6 typed `qm` actions + rollback intent + tests |
| Sandbox execution (git agent) | `crates/cynic-sandbox/src/bwrap.rs` | `build_bwrap_argv`, `LocalBwrapAdapter`, namespace isolation, tests |
| Sandbox execution (cargo build) | `crates/cynic-sandbox/src/cargo.rs` | `BuildSandboxMode`, `CargoSandboxSpec`, `build_cargo_sandbox_argv` — always `--unshare-net`, toolchain read-only, only workdir+target writable, tests |
| Build sandbox wiring | `crates/cynic-node/src/adapters/tokio_build_execution.rs` + `usecases/build_scheduler/prepare.rs` | `BuildSandboxMode::Bwrap` is actually consumed; `resolve_sandbox_mode` decides per build |
| LXC bounded executor | `crates/cynic-lxc/src/{decision,execution}.rs` | `admit`/`execute` via `pct exec`, argv allowlist (no shell), `--apply` required for mutation, breakglass fail-closed, timeout + read-back, tests |
| LXC clone/disk adapters | `crates/cynic-proxmox/src/adapters/proxmox_{clone,disk_extension}.rs` | `LxcCloneOutcome`, `ProxmoxSshDiskExtensionAdapter` |
| Agent bus (coordination plane) | ~~`crates/cynic-agent-bus/`~~ RETIRED | Deleted as zombie crates (`a08d6350`); coordination lives in `cynic-workload`/`pool`/`lifecycle` + protocol claims |
| ~~Agent bus projections~~ (deleted with the family, see above) | — | No surface; do not reference as existing |
| CLI projection | `crates/cynic-cli/src/agent_host_surface.rs` | `cynic agent host plan|observe`, reuses `ProxmoxControlAdapter` |

### 2.2 Missing or incomplete (the real gaps this spec closes)

| Gap | Evidence |
|---|---|
| **Agent-host degradation postures** | `agent_coordination_continuity.rs` covers the *coordination* runtime (MCP/scheduler). Nothing decides what happens when the *hypervisor or VM* is unreachable. |
| **Builder permanent-vs-ephemeral policy** | `BuildRunnerPlacement` is a single declared node. No policy selects a placement mode from observed capacity/risk. |
| **Experimentation protocol** | F0–F5 describe phases but define no plan/receipt contract for ephemeral provision → observe → teardown. |
| **Build sandbox is off by default** | The mechanism exists (`cynic-sandbox/src/cargo.rs` + `tokio_build_execution.rs`), but `resolve_sandbox_mode` defaults to `BuildSandboxMode::Off` (`usecases/build_scheduler/prepare.rs:175`). No policy forces an ephemeral sandbox for a candidate PR build. The mechanism is real; the *policy* is absent. |
| **`cynic-host` guest code** | `crates/cynic-host/src/agent_host/` (1223 LOC) is Proxmox-specific but lives in the L1 monolith (82 813 LOC src, god-object ceiling 25k). |
| **Unconsumed sandbox capability** | `cynic-sandbox/src/netns.rs` and `rapl.rs` are ports with `Null*` implementations only. No consumer. Either implement or prune — do not keep as passive debt (`pruning-policy::no_footprint_justification`). |
| **Agent-bus family (resolved by deletion)** | Removed in `a08d6350`. The wiring/duplication debate is moot — the copy is gone. Residual: confirm no dangling references outside this spec. |
| **Ephemeral LXC has no consumer** | `cynic-proxmox::adapters::proxmox_clone` (`LxcCloneOutcome`) exists but nothing provisions or destroys an ephemeral LXC. This is exactly the missing link for ephemeral placement. |

## 3. Agent-host degradation (three typed postures)

Distinct from coordination degradation. The two compose: an agent may be `RuntimeFenced` (coordination healthy) while the agent host is `FullStop` (VM down). Authority is the intersection — the most restrictive of the two wins.

Schema: `cynic.agent_host.degradation_posture.v1`.

```
enum AgentHostPosture {
    DegradedLocal,    // agent-host unreachable, local bwrap on Cortex available
    FallbackBuilder,  // agent-host unreachable, Builder reachable and admits the workload
    FullStop,         // no isolated execution surface proven
}
```

Selection rule — pure, from observed reachability only (`cynic.agent_host.reachability_observation.v1`):

| Observed | Posture |
|---|---|
| hypervisor reachable AND agent-host VM reachable AND sandbox attested | none (normal path) |
| agent-host unreachable, local bwrap capability observed | `DegradedLocal` |
| agent-host unreachable, Builder reachable AND admits the workload profile | `FallbackBuilder` |
| anything else, including **any unknown** | `FullStop` |

Invariants:
- `FullStop` is the default for `unknown`. An unproven surface never yields a fallback. This mirrors `BlockedUnknown` in `agent_coordination_continuity.rs`.
- `DegradedLocal` never mounts a shared checkout, never receives a writable Forgejo token or Proxmox credential, and keeps `default_deny` egress. It reuses `SandboxStrategy::BwrapEphemeral` and the existing `build_bwrap_argv` invariants.
- `FallbackBuilder` redirects the *workload*, not the authority: the agent's branch/workspace remain private, and the build runs under the ephemeral-sandbox rule of §5. The execution mechanism already exists — `cynic-lxc::{admit,execute}` drives `pct exec` with an argv allowlist (no shell), requires `--apply` for any mutation, and fails closed on breakglass without an incident-authority verifier. `FallbackBuilder` adds the *decision* (is the Builder reachable and does it admit this workload profile?), not a new executor.
- In-flight workloads at the moment of a `FullStop` transition take disposition `quarantine` (`SandboxDisposition::Quarantined`), never silent deletion.
- A posture change is an observable event, not a silent substitution: it emits a `NotificationOutboxEntry` projection (existing contract) and is recorded with the read-back that justified it.

## 4. Builder placement policy (permanent vs ephemeral)

Schema: `cynic.builder.placement_policy.v1` (desired) and `cynic.builder.placement_decision.v1` (observed outcome).

```
enum BuilderPlacementMode {
    Permanent,   // the standing LXC (current observed state)
    Ephemeral,   // provisioned per build from template, destroyed after
    Undecided,   // insufficient observation — keep the standing Builder, do not switch
}
```

The policy is a pure function of observed data, never of a machine name or a fixed preference:

- observed free capacity on the hypervisor (from the existing preflight observer);
- the workload's risk profile (arbitrary egress, unreviewed code, `build.rs`/proc-macros);
- availability of a warm compilation cache;
- measured bootstrap latency of the ephemeral path versus the standing path.

Fail-closed rules:
- No observation → `Undecided` → keep `Permanent`. A policy never switches placement on a hypothesis.
- `Ephemeral` is admissible only after F0 evidence records: template present, storage pool present, bootstrap latency, and a teardown read-back that proved a prior ephemeral instance was fully destroyed.
- The decision and its evidence are recorded as a receipt; the mode may change only with the receipt present.
- This does not retire the permanent Builder. Retirement is a separate, evidence-gated decision (§7).

Placement is a decision, not a mutation: no `qm`/`pct` argv is emitted by the policy. Actuation stays behind the `cynic-proxmox` adapters and an explicit `--apply`.

The ephemeral path has a ready actuator: `cynic-proxmox::adapters::proxmox_clone` (`LxcCloneOutcome`, `ProxmoxSshCloneAdapter`) clones an LXC from template, and `proxmox_disk_extension` resizes it. What is missing is the orchestrating decision and the teardown obligation (§5), not the clone mechanism.

## 5. Ephemeral execution and experimentation protocol

Schema: `cynic.agent_host.experiment_plan.v1` and `cynic.agent_host.experiment_receipt.v1`.

```
desired state (registry intent)
  → observe        (F0: capacity, VMID free, template, pool, bridge, latency)
  → reconcile      (pure: provision | reuse | blocked | unknown)
  → bounded action (explicit --apply, typed argv, rollback intent, stop condition)
  → read-back proof (identity, image provenance, principal, cgroup, network, teardown)
  → evolve by data (receipt feeds the next placement decision)
```

Rules:
- An experiment binds: baseline digest, allowed paths, action and wall-time budget, the declared isolation profile, expected validation, and a rollback intent — mirroring `HumanScopedFallbackProposal` in `docs/specs/agent-degradation-and-execution-authority.md`.
- Every ephemeral resource carries a teardown obligation with read-back. Retention is explicit (`SandboxDisposition::{Retained,Quarantined,EligibleForHumanReview}`), never implicit. No automatic `--force`.
- The Builder LXC is not exempt from isolation: a candidate compilation (`build.rs`, proc-macros) runs inside an ephemeral sandbox within the LXC — rootfs read-only, `default_deny` network. The LXC alone is not proof of build-workload isolation. The argv constructor (`cynic-sandbox::cargo::build_cargo_sandbox_argv`) already guarantees `--unshare-net` with only workdir+target writable; the missing piece is a policy that refuses a candidate build unless `BuildSandboxMode::Bwrap` is set (today `resolve_sandbox_mode` may return `Off`).
- The experiment receipt is durable and secret-free (`no_secrets_verified`), and is the only input that may move a placement decision.

## 6. De-bloating `cynic-host` (into existing crates only)

No new crate is created. Every move targets a crate that already owns the responsibility, per `pruning-policy.json::one_responsibility_one_crate`.

| Guest in `cynic-host` | Target | Justification |
|---|---|---|
| `src/agent_host/` (1223 LOC: Proxmox REST/SSH observe + `qm` plan) | `cynic-proxmox` (L1, already "owns every Proxmox-specific adapter") | Provider-specific responsibility on a different schedule. Also removes the current `cynic-proxmox → cynic-host` dependency edge, which is backwards for a provider crate. |
| `src/privilege/pve_helper.rs` (install-site choreography + sudoers membrane) | evaluate: shared catalog stays; install-site policy may stay in host | The capability list is already `infra/capabilities/catalog.json`. Do not move blindly — read-back the callers first. |
| `src/bridge.rs` (sandbox_root references) | `cynic-sandbox` if it is sandbox policy; else stays | Verify before moving. |

Consolidation candidates (not extraction — fusion, per `pruning-policy::deduplication_before_new_abstraction`):

- **Agent-bus `Claim`/wiring (RESOLVED by deletion `a08d6350`)** — the family is gone, so the duplication and wiring debates are moot. Do not revive. Residual: grep the tree for dangling references.
- **`cynic-sandbox::{netns,rapl}`** — ports with only `Null*` implementations. Implement or prune; no consumer today.

Constraints:
- Pure types (`AgentHostDesired`, `AgentSandboxAllocation`, …) stay in `cynic-protocol`. Only the Proxmox-specific adapters and the `qm` plan move.
- `crates/cynic-cli/src/agent_host_surface.rs` repoints its imports from `cynic_host::agent_host::*` to `cynic_proxmox::agent_host::*`.
- Extraction happens organically: move what is stable and proven first; the rest follows the evidence.
- No net growth of `cynic-host`. The ratchet (`crate_loc_ratchet`, god-object ceiling 25k in `contracts/foundation.json`) must only improve.

## 7. Bottom-up implementation order

1. **`cynic-protocol`** — add the pure types and predicates: `AgentHostPosture` + `reachability_observation`; `BuilderPlacementMode` + decision; `experiment_plan`/`experiment_receipt`. Module-level, inside existing files or a sibling module in the same crate. Tests: posture selection including `unknown → FullStop`; placement fail-closed; experiment binding and teardown obligation.
2. **`cynic-proxmox`** — host the moved `agent_host` module (observe + plan). No behavior change, imports only.
3. **`cynic-host`** — remove the moved module, repoint consumers, confirm no net growth.
4. **`cynic-node`** — make the candidate-build sandbox policy explicit: a candidate PR build must not silently take `BuildSandboxMode::Off`. The executor and argv constructor already exist (`tokio_build_execution.rs`, `cynic-sandbox::cargo`); this step is policy, not new machinery.
5. **`cynic-lxc` / `cynic-proxmox`** — orchestrate ephemeral LXC provision→observe→teardown using the existing `proxmox_clone` + `cynic-lxc` executor. Add the teardown obligation and read-back, not a new executor.
6. ~~**`cynic-agent-bus`**~~ — DONE via deletion (`a08d6350`); verify no dangling references tree-wide.
7. **`cynic-cli`** — thin projections: `cynic agent host posture`, `cynic builder placement`, `cynic agent host experiment plan|receipt`. No policy duplicated in the CLI.
8. **Conformance** — hypervisor loss, VM loss, Builder loss, unknown reachability, ephemeral teardown read-back, placement switch without receipt (must be refused), candidate build with sandbox `Off` (must be refused).

## 8. Non-negotiable boundaries

- No fallback, posture, or placement ever creates runtime authority. Degradation and placement are decisions; authority stays in `one_shot_capability` / `capability_lease`.
- No secret, token, or credential is ever transported in a receipt, notification, or experiment plan. Presence is reported, never the value.
- `unknown` is data: report `unknown` or `blocked`, never fabricate convergence.
- The system stays functional when Cortex is off. If a design requires Cortex, the design is incomplete.
- No mutation of fleet, network, secret, or service without explicit human consent and a rollback intent.

## 9. Acceptance criteria

1. An agent runs under an attested isolation surface, or the posture is an explicit `FullStop` — never an implicit fallback.
2. The Builder placement mode is backed by a receipt; a switch without a receipt is refused.
3. Two parallel agents hold independent VM/sandbox/worktree/Git admin, proven.
4. Every ephemeral resource has a teardown read-back, or an explicit retained disposition.
5. `cynic-host` does not grow; the moved module lives in `cynic-proxmox` with no behavior change.
6. Cortex can be stopped; Forgejo, agent-host, Builder and their proofs remain observable.
