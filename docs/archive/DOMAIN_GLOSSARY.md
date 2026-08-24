# CYNIC Domain Glossary

Canonical vocabulary for CYNIC runtime concepts. Each term links to its canonical
Rust definition. Status labels: **observed** (exists in code), **proposed**
(design only, see `.hermes/plans/2026-08-24_universal-time-energy-task-roi-plan.md`).

## A

### Agent Citizen
- **Definition**: A native CYNIC scheduler workload running under
  `cynic.agent_workload_envelope.v1` with monotonic budgets and explicit proof contracts.
- **Contrast**: `baseline` mode agents (Hermes/Codex/Pi/CLI) cooperate via claims.
- **Code**: `crates/cynic-protocol/src/agent_workload.rs` — observed

## C

### CapacityObservation
- **Definition**: Point-in-time observation of available capacity (hardware profile +
  system metrics + metabolic state), feeding EnergyBudget admission.
- **Status**: proposed (plan §4)
- **Existing inputs**: `cynic-inference/src/domain/hardware_profile.rs::HardwareProfile`,
  `cynic-node/src/domain/system_observation.rs::SystemMetricsSample` — observed

### ConcurrencySlot
- **Definition**: A parallel execution place of an inference backend (`--parallel`),
  with priority queueing. NOT a time unit.
- **Code**: `crates/cynic-inference/src/domain/slot_tracker.rs`, `slot_semaphore.rs`,
  `inference_queue.rs` — observed

## D

### Disk Observer
- **Definition**: Filesystem/mount/cache observation via portable ports.
- **Contract**: `crates/cynic-protocol/src/disk.rs` — observed
- **Adapters**: `crates/cynic-host/src/adapters/local_disk.rs`, `remote_disk.rs`

### DomainTask
- **Definition**: Structured async task partitioned by domain
  {Inference, Infra, CodeAudit, Cortex, Governance, Nightshift} with priority
  Critical=4..Background=0, claim/complete lifecycle, timeout_secs.
- **Schema**: `cynic.domain_task.v1`
- **Code**: `crates/cynic-protocol/src/domain_tasks.rs`; queue:
  `cynic-inference/src/domain/domain_task_queue.rs` (JSONL) — observed

### DeploymentEpoch / ReleaseEpoch
- **Definition**: Content-addressed release unit (commit+desired_state+scope)
  advancing through phases Proposed→…→Completed one observation at a time.
- **Note**: distinct from temporal Epoch; rename to ReleaseEpoch proposed.
- **Code**: `crates/cynic-protocol/src/deployment_epoch.rs` — observed

## E

### EnergyBudget
- **Definition**: Typed allocation per window {cpu_ms, ram_bytes_s, io_bytes,
  net_bytes, inference_tokens, consented human attention}; admission requires
  reservability against the latest CapacityObservation.
- **Status**: proposed (plan §4)

### E-Score
- **Definition**: Contextual, versioned, contestable interpretation of attributed
  evidence. Never a universal measure of human worth (AGENTS.md).
- **Code**: `crates/cynic-intel/src/escore/` — observed

### Epoch (temporal)
- **Definition**: Discrete logical time unit; incremental, depends only on runtime
  progression, never on physical clock. Third axis of convergence space.
- **Code**: `crates/cynic-protocol/src/temporal.rs::Epoch`;
  `convergence/causal.rs::CausalConvergenceEpoch` — observed

## F

### Fleet
- **Definition**: Federation of node-local agent trees (not a global orchestrator).
- **Code**: `crates/cynic-protocol/src/federation.rs` — observed

## H

### HarnessSample
- **Definition**: Measured inference run sample: tokens/s in/out, wall_time_ms,
  RSS/available/VRAM/GTT memory, event stream stats.
- **Code**: `crates/cynic-inference/src/application/agent_harness/types.rs:184` — observed

## K

### Kyon
- **Definition**: Protective-signal vocabulary for the protected person:
  MirrorSignal (severity/uncertainty bps) → ShieldPolicy → ProtectivePlan;
  federated disclosure only via explicit ConsentGrant; RawPrivate never federates.
- **Code**: `crates/cynic-protocol/src/kyon/{types,decision}.rs` — observed
- **Scenarios**: `crates/cynic-askesis/src/kyon_scenarios/` — observed

## M

### MemoryReport
- **Definition**: 4-level memory observation: physical / volume (swappiness,
  hugepages) / logical usage / pressure stalls (PSI).
- **Schema**: `cynic.host_memory_report.v1`
- **Code**: `crates/cynic-protocol/src/memory.rs` — observed

### MetabolicState
- **Definition**: Host energy state {HighPerformance, Standard, Survival} derived
  from CPU/RAM thresholds. Currently host-only port, not yet a protocol contract.
- **Code**: `crates/cynic-host/src/ports/metabolism.rs` — observed (fragment)

## P

### PoH Checkpoint Batch
- **Definition**: Merkle-compressed batch of offchain evidence digests settled as
  ONE Solana instruction (`record_poh_checkpoint`). Batches by events, has an
  epoch field but NO slot/duration concept.
- **Schema**: `cynic.poh_checkpoint_batch.v1`
- **Code**: `crates/cynic-protocol/src/anchor_client.rs` — observed

### Proof
- **Definition**: Post-action read-back evidence; freshness bounded by max age
  (`DEFAULT_PROOF_FRESHNESS_MAX_AGE_MS = 5 min`), never by command success.
- **Code**: `crates/cynic-host/src/runtime_metrics.rs:11` — observed

## R

### ROI (multi-dimensional)
- **Definition**: Contextual value/cost assessment per ValueFrame; dimensions are
  declared per frame (skill_reuse_gain, KPI readiness, escore PoA), costs are
  measured receipts (energy, wall time). No universal score across contexts.
- **Status**: proposed synthesis; existing primitives: `autoresearch.rs::skill_reuse_gain`,
  `outcomes.rs::KpiScoreReport` — observed (fragments)

## T

### TaskReceipt
- **Definition**: Completion record carrying result + actually consumed resources +
  read-back proof refs. Extends DomainTask.result_evidence.
- **Status**: proposed enrichment (plan §6 step 4)

### TickWindow
- **Definition** (proposed): Universal budgeted window contract — logical epoch
  number + layer + wall-time budget + capacity ref + status. The fractal time
  primitive every layer instantiates (kernel tick, host reconciliation, node
  workload step, fleet rollout phase, lab benchmark).
- **Status**: proposed (plan §4-§5); replaces the superseded task_pipeline matrices

## U

### Unknown
- **Definition**: First-class outcome meaning "not measured", contributing to
  residual risk; never coerced to zero, never fabricated as convergence.
- **Code**: `crates/cynic-protocol/src/convergence.rs::ConvergenceStatus`;
  `runtime_metrics.rs` rules ("unknown metrics are null") — observed

## V

### ValueFrame
- **Definition**: Declared value function for one context (outcome ids + scoring
  rule); consumes TaskReceipts and emits contestable dimensional ROI.
- **Status**: proposed (plan §4)

## Historical note

The former entries "Mempool (SQLite)", "Slot (200→150→100→50→350 ms phase
progression)" and "Pi dev harness = qemu/libvirt" were removed on 2026-08-24:
they described design suppositions, not observed code. See the supersession
note in `.hermes/plans/2026-08-24_universal-time-energy-task-roi-plan.md`.
