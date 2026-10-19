# Scheduler Truths T1–T5 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the single-VM build scheduler observable (stage timings, terminal reasons), capacity-gated, and bounded (terminal archive), and surface the manual-build bypass trail.

**Architecture:** Follow the existing proof pattern (`persist_cancellation_proof`, `persist_renewal_proof` in `build_scheduler/mod.rs`): every new fact lands in a persisted, digestible proof, never in logs alone. Pure decisions stay in `cynic-protocol`/`cynic-build`; orchestration stays in `cynic-node` usecases. TDD, one task = one gate.

**Tech Stack:** Rust, existing crates only (no new dependencies).

**Spec:** Truths T1–T5 from the 2026-10-14 causal analysis (queue wait ≤1 tick, 47/48 blocked causeless, bypass loop, OOM caution, O(records) scan). Measurements: 48 store records, exec bimodal (≤61.5s vs ~30d stale), zero real scheduler builds completed, suite SIGKILL under load, queue scan 40s/12 entries.

## Global Constraints

- Builder SSOT (`10.10.10.119`): all proof runs on the builder, never cortex-only.
- `git add -A` / `git add .` forbidden: stage exact paths only.
- Isolated worktree per branch, base `origin/main`.
- French operator dialogue, 100% English code/tests/docstrings/commits.
- No new top-level CLI families without domain proof (nest under owning surface).

## Review Focus

- A second live build settling while the archive task moves its record dir: the archive must re-verify terminal state at mutation time (same read-back discipline as reclaim).
- Clock skew between commit timestamps and the scheduler clock: durations must never go negative in proofs (clamp, flag).
- `capacity.rs` carries pre-existing French comments (franglais debt): new code stays 100% English, no drive-by reformat.
- Saturated-tier refusal must never strand a workload forever: refusal is per-iteration, the record stays queued.
- Branch/commit admission fields must stay `Option` with legacy default so old envelopes still verify.

---

### Task 1: Persist the settlement proof (T2)

**Files:**
- Modify: `crates/cynic-node/src/usecases/build_scheduler/mod.rs` (add `persist_settlement_proof`, call it where `_receipt` is dropped, ~line 342)
- Test: `crates/cynic-node/src/usecases/build_scheduler/` (follow existing test module pattern in `mod.rs` or `lifecycle.rs`)

**Interfaces:**
- Consumes: `SettledParcelReceipt` (`settled_at_ms`, `final_record_digest`), `AgentCompletionStatus`, `unknowns: Vec<String>`, `elapsed: Duration`, `measured_cpu_ms: Option<u64>` — all already in scope at the call site.
- Produces: `persist_settlement_proof(workload_id: &str, status: &AgentCompletionStatus, unknowns: &[String], elapsed: Duration, measured_cpu_ms: Option<u64>, settled_at_ms: u64)` writing via `agent_execution_runtime_observer::persist_capabilities(&["scheduler_settlement"], &[evidence_ref])` where `evidence_ref` = `scheduler-settlement:{digest}` of the JSON proof `{workload_id, status, unknowns, elapsed_ms, measured_cpu_ms, settled_at_ms}`.

- [ ] **Step 1: Write the failing test** — `settlement_proof_persists_reasons_and_timings`: build a fake receipt + Blocked status with two unknowns, call `persist_settlement_proof`, assert the persisted proof JSON contains both unknowns, `elapsed_ms`, and `status: "blocked"`. (Mirror how existing tests assert `persist_cancellation_proof` output, if any; otherwise assert via the observer's read-back.)
- [ ] **Step 2: Run it to verify it fails** — Run: `cargo test -p cynic-node --lib build_scheduler::` on builder. Expected: FAIL, function not defined.
- [ ] **Step 3: Implement `persist_settlement_proof` in `mod.rs`** next to `persist_cancellation_proof`; replace `let _receipt` with a call that passes `status`, `unknowns`, `started.elapsed()`, `measured_cpu_ms`, `receipt.settled_at_ms`. Keep the existing `tracing::info!`.
- [ ] **Step 4: Run tests to verify pass** — same command. Expected: PASS.
- [ ] **Step 5: Commit** — `git add crates/cynic-node/src/usecases/build_scheduler/mod.rs` + test file; message `feat(node): persist scheduler settlement proof with reasons and timings`.

### Task 2: Stage timings + queue wait in the proof (T1)

**Files:**
- Modify: `crates/cynic-node/src/usecases/build_scheduler/mod.rs` (execute path: timestamp prepare/execute boundaries), same proof writer from Task 1 (extend its JSON, additive fields only).

**Interfaces:**
- Consumes: `persist_settlement_proof` from Task 1 (extended signature: add `prepare_ms: u64, execute_ms: u64, queue_wait_ms: u64`).
- Produces: proof JSON gains `prepare_ms` (claim→prepare done), `execute_ms` (execute start→finish), `queue_wait_ms` (`first_claim_ms - committed_at_ms`, clamped ≥ 0 with `queue_wait_clamped: bool` flag when skew detected).

- [ ] **Step 1: Write the failing test** — extend Task 1's test: assert `prepare_ms`, `execute_ms`, `queue_wait_ms` present and non-negative; assert a negative raw wait yields `0` with `queue_wait_clamped: true`.
- [ ] **Step 2: Run to verify it fails** — Expected: FAIL on missing fields.
- [ ] **Step 3: Implement** — capture `Instant` at prepare start/end and execute start in the settle path; compute `queue_wait_ms` from `latest.committed_at_ms` vs first claim time (claim revision `committed_at_ms` — read from the durable record chain already in scope).
- [ ] **Step 4: Run tests** — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(node): stage timings and queue wait in settlement proof`.

### Task 3: Capacity gate at admission (T4)

**Files:**
- Modify: `crates/cynic-node/src/usecases/build_scheduler/mod.rs` (`run_iteration`: measure → assess → skip claim when Saturated).
- Test: same module; pure cases via `assess_load_tier` with fabricated `CpuReport`/`MemoryReport` already exist in protocol — reuse those constructors.

**Interfaces:**
- Consumes: `cynic_protocol::assess_load_tier(cpu: Option<&CpuReport>, memory: Option<&MemoryReport>, policy: &LoadPolicy) -> CapacityAssessmentV1` (existing); host_substrate readers that produce the reports on the builder (existing `cynic-host` observers — wire the lightest local read; if no local reader fits `cynic-node`'s dependency direction, measure via `/proc` through the existing `meminfo_reader`/`remote_cpu` adapters only if already depended upon, else record `cpu_report_missing`/`memory_report_missing` unknowns and proceed per policy).
- Produces: per-iteration behavior — `LoadTier::Saturated` ⇒ skip claiming this tick (record stays queued, `tracing::info!` with tier); otherwise proceed; tier string + report digests passed into `persist_settlement_proof` (extend JSON additively: `load_tier`, `capacity_unknowns`).

- [ ] **Step 1: Write the failing test** — `saturated_tier_skips_claim`: Saturated assessment ⇒ `run_iteration` (or the extracted gate function `admit_claim_under_capacity`) returns skip without touching the store.
- [ ] **Step 2: Run to verify it fails** — Expected: FAIL.
- [ ] **Step 3: Implement** — extract `admit_claim_under_capacity(assessment) -> bool` (pure, tested); call in `run_iteration` before `queue_ordered_ids` loop; `Standard` + unknowns ⇒ proceed (policy default, never refuse on unmeasured).
- [ ] **Step 4: Run tests** — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(node): capacity-gated build admission`.

### Task 4: Terminal record archive (T5)

**Files:**
- Modify: `crates/cynic-node/src/ports/agent_workload_store_port.rs` (add `archive_terminal(workload_id: &str, older_than_ms: u64, now_ms: u64) -> Result<Option<String>, AgentWorkloadStoreError>` returning the archive path or None when ineligible).
- Modify: `crates/cynic-node/src/adapters/local_agent_workload_store.rs` (impl: re-read latest revision, verify terminal via `AgentWorkloadState::terminal()`, verify age, move dir to `<root>/archive/`).
- Test: adapter tests with fabricated `revision-0000000000000000000N.json` files (production filenames — see the `.json` lesson).

**Interfaces:**
- Consumes: `AgentWorkloadState::terminal()` (existing, `cynic-workload`).
- Produces: `archive_terminal` on the port + impl. Wiring (CLI surface or loop hook) is deliberately OUT of this task — next task.

- [ ] **Step 1: Write failing tests** — terminal+old ⇒ archived (dir moved, path returned); live ⇒ None; missing ⇒ None; terminal+fresh ⇒ None.
- [ ] **Step 2: Run to verify they fail** — Expected: FAIL, method missing.
- [ ] **Step 3: Implement** the port method + adapter (move = copy+remove? use `fs::rename` same filesystem, verify absence after move by read-back).
- [ ] **Step 4: Run tests** — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(node): terminal workload record archive in store port`.

### Task 5: Archive wiring — explicit CLI surface (T5)

**Files:**
- Modify: `crates/cynic-cli/src/build_surface/` (nest under existing build family, e.g. `cynic build store purge --apply`; exact arm name chosen at implementation under the owning family).
- Test: CLI-level test proving dry-run lists eligible archives and `--apply` moves them, with proof output.

**Interfaces:**
- Consumes: `archive_terminal` from Task 4.
- Produces: dry-run report (eligible workload ids + ages) and applied report (moved paths + absence read-backs), same JSON-proof style as `reclaim`.

- [ ] **Step 1–5:** TDD as above. Commit `feat(cli): terminal workload archive surface`.

### Task 6: Bypass trail in reclaim observation (T3, measurable half)

**Files:**
- Modify: `crates/cynic-host/src/adapters/local_build_cache.rs` (observation gains `unlinked_checkouts: Vec<String>` — checkouts under the evidence scope whose name matches no workload record id in the configured store).
- Test: adapter tests (fabricate checkout dir + store with/without matching `revision-*.json`).

**Interfaces:**
- Consumes: existing scope + store-relative config already in the adapter.
- Produces: `CacheObservation.unlinked_checkouts` (additive field; pure decision untouched — observation only).

- [ ] **Step 1–5:** TDD as above. Commit `feat(host): surface unlinked evidence checkouts in reclaim observation`.

### Task 7: Branch + commit at admission (T3, fair-queue key)

**Files:**
- Modify: `crates/cynic-workload/src/types.rs` (envelope: `source_commit: Option<String>`, `branch: Option<String>`, serde default) + `crates/cynic-build/src/build/agent_workload.rs` (compose fills them from the submit request; verify keeps legacy `None` passing).
- Modify: `crates/cynic-build/src/build_queue.rs` (`QueuedCandidate` gains the two optional fields; ordering UNCHANGED this task).
- Tests: compose/verify round-trips + queue plan with the new fields present.

- [ ] **Step 1–5:** TDD as above. Commit `feat(workload): branch and commit identity at admission`.

## Self-Review

- Spec coverage: T1→Tasks 1–2, T2→Task 1, T3→Tasks 6–7, T4→Task 3, T5→Tasks 4–5. All covered.
- Type consistency: `persist_settlement_proof` signature grows additively across Tasks 1–3 — final shape fixed here: `(workload_id, status, unknowns, elapsed, measured_cpu_ms, settled_at_ms, prepare_ms, execute_ms, queue_wait_ms, queue_wait_clamped, load_tier, capacity_unknowns)`. Prefer a `SettlementProofFields` struct if the parameter list offends the reviewer at Task 3.
- Proportion: 7 tasks, each independently gateable; order executable 1→2→3→6→7→4→5 or in listed order.
