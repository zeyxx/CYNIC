# Goal: Data-Centric Convergence — Observation, Decision, Action, Proof

- **Schema**: `cynic.goal.data_centric_convergence.v1`
- **Date**: 2026-10-01
- **Status**: `proposed` (awaiting ratification)
- **Orchestrator**: Hermes-Agent
- **Predecessor**: `docs/goals/2026-10-01-closed-loop-restoration-and-distillation.md`

---

## 1. Why this goal exists

Three anomalies on the builder exposed the same defect class three times: **a
declared step with no executor**.

1. `infra/registry.json` declared a cache-reclaim policy. The pure decision
   existed and was tested. The executor for the non-canonical step did not, so
   the canonical reclaim was unreachable and the builder filled to 100% while
   the registry described the answer.
2. `SshBuildRunner` creates one checkout per commit and never removes any. 37
   checkouts reached 18G. No retention policy is declared anywhere.
3. `plane.rs` and `topology_generate.rs` were orphan modules — declared by
   nothing, reachable by nothing, until a ratchet caught them.

A policy that cannot be obeyed reads, from the operator's seat, exactly like a
policy that is being obeyed. That is the failure mode this goal addresses.

The fix is not more executors. It is a single, explicit chain from **observation**
to **decision** to **action** to **proof**, where every step is data and every
step is auditable. Once that chain exists, a declared step without an executor
becomes a detectable gap rather than a silent one.

---

## 2. Current state (mapped, not assumed)

What already exists and is data-centric:

| Domain | Crate | Existing surface |
|---|---|---|
| LLM routing | `cynic-inference` | `RoutingCalculator` (latency/success-driven), `InMemoryModelRegistry`, `agent_harness` |
| Hardware | `cynic-host-probes` | `hardware.rs`, `llm.rs`, `environment.rs` — pure observation |
| Coordination | `cynic-agent-bus` | `AgentExecutor` (provider-agnostic), `CoordinatorReasoner` (plan → DAG) |
| Proof | `cynic-askesis` | Append-only hash chain, tamper-evident |
| Build | `cynic-host` | `SshBuildRunner`, cache reclaim (now executed), topology |
| Network | `cynic-network` | Mesh, firewall, dynamic registry resolution |

What is missing:

1. **No unified observation plane.** Probes exist but do not feed a shared store.
   `RoutingCalculator` consumes `DogPerformance`; nothing supplies it continuously.
2. **No data-centric coordination between domains.** `AgentExecutor` does not route
   on observations. `CoordinatorReasoner` does not consume hardware/LLM/network
   metrics. Domains are isolated.
3. **No dynamic policy plane.** `registry.json` is static. Routing, build and
   deploy policies live in separate files or in code.
4. **No feedback loop.** `prune_stale_backends` exists and nothing calls it. The
   build runner creates checkouts and nothing reclaims them.

---

## 3. Target architecture

```
OBSERVATION PLANE
  host-probes      -> hardware, llm, environment metrics
  build-runner     -> duration, cache hit rate, disk usage, checkout count
  network          -> mesh health, latency, throughput
  agent-bus        -> success rate, task duration, capability grants
  Format: Observation { source, kind, value, observed_at }
  Store: append-only (AskesisLog) + in-memory query cache
        |
        v
DECISION PLANE
  pure functions: Observation -> Decision
  RoutingDecision   (extends RoutingCalculator)
  BuildDecision     (extends the existing cache policy)
  RetentionDecision (new: checkout/artifact lifecycle)
  AgentDecision     (new: agent selection by capability + health)
  DeployDecision    (new: node selection by health)
        |
        v
ACTION PLANE
  SshBuildRunner, AgentExecutor, LocalBuildCacheAdapter, NetworkAdapter
  Each action emits a new Observation -> loop closes
        |
        v
PROOF PLANE
  Every decision and every action appended to AskesisLog
  who decided what, when, from which observations
```

---

## 4. Decomposition into supervised milestones

### Milestone 1 — Observation plane (P0)

- **Action**: Create `cynic-observability` (L1). Ports `ObservationSink`
  (`record`) and `ObservationQuery` (`query`). Adapters `InMemoryObservationStore`
  and `AskesisObservationStore`. Existing probes become producers.
- **Read-back proof**: `cargo test -p cynic-observability` green; a probe run
  writes observations readable by a query with the same filter.
- **Owning boundary**: new crate; requires operator approval (new crate = ask first).

### Milestone 2 — Decision plane (P1)

- **Action**: Create `cynic-decision` (L2). Port `DecisionEngine`:
  `decide(&[Observation]) -> Decision`. Pure functions, zero I/O. Existing
  `RoutingCalculator` and the build cache policy become `Decision` impls rather
  than parallel paths.
- **Read-back proof**: `cargo test -p cynic-decision` green, including
  falsifiability runs (remove a rule, a test must fail naming itself).
- **Owning boundary**: new crate; requires operator approval.

### Milestone 3 — Close the loop (P1)

- **Action**: Wire `host-probes -> ObservationSink -> DecisionEngine -> Action`.
  Each action emits a new Observation. The builder's own disk pressure becomes
  the first closed loop: observe free bytes, decide reclaim, act, re-observe.
- **Read-back proof**: on the builder, the disk-pressure loop reduces usage below
  the declared minimum and the second observation proves it — with no manual step.
- **Owning boundary**: `cynic-host` adapters + `cynic-cli` surface.

### Milestone 4 — Retention as declared policy (P2)

- **Action**: Declare checkout/artifact retention in `infra/registry.json`
  (`checkout_retention`), and execute it through the same decision plane. The
  build runner stops creating unbounded checkouts.
- **Read-back proof**: after N triggers, checkout count stays at or below the
  declared bound, read back from the builder.
- **Owning boundary**: `infra/registry.json` + `cynic-host`.

---

## 5. Constraints

- New crates require explicit operator approval (AGENTS.md: ask first).
- Zero-I/O domain core: decisions are pure; I/O stays behind ports.
- Strict downward DAG: L1 must never depend on L2 or L3.
- No new contract unless a stable boundary cannot be expressed in Rust types,
  tests, registry, or `foundation.json`.
- Every milestone closes with a machine-readable read-back, never a claim.
- Unknown is data: report `unknown` or `blocked`, never fabricate convergence.

---

## 6. Status ledger

| Milestone | Status | Evidence |
|---|---|---|
| 0. Cache reclaim executor | `done` | `faa3a454`, 6 tests, builder 6/6 ratchets |
| 0. Reclaim CLI surface | `done` | `f1f796e5`, dry-run on Cortex and builder |
| 1. Observation plane | `proposed` | — |
| 2. Decision plane | `proposed` | — |
| 3. Closed loop | `proposed` | — |
| 4. Retention policy | `proposed` | — |

---

## 7. Fourth anomaly: the proof log could not say no

Found while auditing my own false proof claims on 2026-10-03. This one is the
sharpest instance of the defect class, because it made the defect class
invisible.

### What happened

`SshBuildRunner::trigger` returned `Err(BuildError::AdmissionBlocked(..))` when
the remote build failed. The CLI surface then did:

```rust
let proof = runner.trigger(&trigger).map_err(|e| e.to_string())?;   // <- exits here
let rendered = proof.to_json();
append_proof(&log_path, &rendered);
```

The `?` returned **before** the append. So the proof log retained only
converged builds. Two very different situations were indistinguishable in it:

| Situation | Proof log |
|---|---|
| Commit never tested | no line |
| Commit tested, **build failed** | **no line** |
| Commit tested, converged | one line |

`PipelineStatus::Blocked` existed in the type and was **never constructed** —
its only occurrence in the workspace was the `Display` arm. `LayerStatus::Failed`
likewise. The shape of failure was fully modelled and never populated.

No gate reads the proof log either: `cynic-release` and `cynic-lifecycle`
contain zero references to `converged`. So the log was decoration that could
only confirm.

### Why it mattered here

I reported `2c0ea083` as proven. It does not compile under the build runner's
configuration (`unused import: ServiceProbeSpec`, promoted to an error because
`[workspace.lints.rust] unused_imports = "deny"`). Reading "no line in the log"
as "nothing to report" is exactly the mistake the log's shape invited.

### The correction (tranche 1, `done`)

- `BuildVerdict { Converged, Blocked { broken_layer, reason } }` — the runner's
  verdict on a commit.
- `BuildError` narrowed to the cases where **no verdict exists**:
  `InvalidCommit`, `RunnerUnreachable`, `ConfigurationRefused`. `AdmissionBlocked`
  and `ProofUnparseable` are gone: both described observed outcomes.
- `BuildProofBundle.verdict_reason` carries the compiler output; `to_json`
  escapes it (unescaped output would produce an unparseable line, which the
  reader silently skips — the same disappearance).
- The CLI appends on **every** verdict, then sets the exit code from
  `proof.status`. A blocked build exits non-zero with the proof retained.
- Tests: 29 pass. `trigger_records_a_failed_build_as_a_blocked_verdict` and
  `blocked_proof_survives_json_round_trip` replace `trigger_blocks_on_build_failure`,
  which had asserted `Err` and so encoded the defect as intent.
- Falsifiability run: restoring the original `Err` return fails 2 tests naming
  themselves; dropping `verdict_reason` from `to_json` fails 1.

### What remains (tranche 2)

The log can now say no. Nothing yet **requires** it to be read. Tranche 2 adds
the consumer: a `--status blocked` filter on `cynic build proofs`, and a release
gate that refuses to promote a commit whose latest proof is `Blocked` or absent.

Milestone 3 (the owner of the promotion decision) is deferred: whether the gate
belongs in `cynic-release` or where `foundation.json` declares the authority is
a question to study, not to guess.
