# Fleet Supervisor Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace cold probing at placement time with a supervised capacity registry: runners are observed on a schedule, placement reads the registry, drift alerts the operator, remediation stays a signed breakglass gesture.

**Architecture:** Pure decisions in `cynic-protocol` (snapshot type + usable/muted/degraded decide). Registry `infra/fleet-capacity.json` under contract. `cynic-node` owns the reconcile loop (observe → registry → alert). `cynic-build/src/place.rs` becomes a registry reader. SSH is used only by the existing probe functions on schedule (phase A) and by the signed remediate gesture; mesh-pushed snapshots (phase B) remove even that.

**Tech Stack:** Rust, serde_json registries, existing `SshBuildRunner` probes, `NotificationOutbox` + `network_notify_surface` for alerts, builder `check` + `test` proofs per task.

**Spec:** This plan IS the spec (design agreed 2026-10-22: watcher option 1, SSH as breakglass, QUIC/mesh as house transport). No separate design doc.

## Global Constraints

- Read-only arms never mutate and never take `--yes`.
- Every mutation arm requires explicit `--yes`.
- No new top-level CLI command: new arms nest under the owning family (`fleet`).
- Pure logic in domain modules; I/O behind ports in adapters.
- Builder is the build/test SSOT; never build on cortex.
- Each task ends with a builder proof naming its exact SHA.

## Review Focus

- A stale snapshot must read as muted, never as healthy; the test that pins it lives in Task 2.
- No cold-probe fallback may re-enter `place.rs`; the refusal message must name the `fleet observe` command (Task 4).
- Remediation must never self-trigger; the gesture requires `--yes` and a human seat (Task 6).
- Probe tokens travel via `cynic_secrets::env_var`, never argv-echoed in logs (Task 3).
- Registry schema changes must update the contract file in the same task (Tasks 2, 3).

---

### Task 1: Capacity snapshot type + pure decide in `cynic-protocol`

**Files:**
- Create: `crates/cynic-protocol/src/runner_capacity.rs`
- Modify: `crates/cynic-protocol/src/lib.rs` (module declaration)
- Test: in-file `#[cfg(test)]` in `runner_capacity.rs`

**Interfaces:**
- Consumes: nothing new (serde only).
- Produces: `pub struct RunnerCapacitySnapshot { runner_id: String, observed_unix_s: u64, free_ram_bytes: Option<u64>, disk_avail_bytes: Option<u64>, state_root: Option<String> }`, `pub enum RunnerUsability { Usable, Degraded(String), Muted(String) }`, `pub fn decide_runner_usability(snapshot: &RunnerCapacitySnapshot, now_unix_s: u64, floor_ram_bytes: u64, staleness_secs: u64) -> RunnerUsability`.

- [ ] **Step 1: Write failing tests** `snapshot_missing_disk_is_muted`, `stale_snapshot_is_muted`, `below_floor_is_degraded`, `fresh_above_floor_is_usable`, with `staleness_secs = 600`, `floor_ram_bytes = 2147483648`.
- [ ] **Step 2: Run to verify they fail** Run: builder `test` phase `--package cynic-protocol --filter runner_capacity`. Expected: compile fail (module missing).
- [ ] **Step 3: Implement** the struct, enum, and decide fn with the exact names above.
- [ ] **Step 4: Run tests to verify they pass** Run: same proof. Expected: 4 passed, 0 failed.
- [ ] **Step 5: Commit** with the proof SHA in the message.

### Task 2: Capacity registry + contract

**Files:**
- Create: `infra/fleet-capacity.json` (seed: `{ "schema": "cynic.fleet.capacity.v1", "snapshots": [] }`)
- Create: `contracts/fleet-capacity.v1.json` (schema for the registry)
- Test: manual `jq empty` + `python3 -c json.load` in the step (no Rust test; registry is data)

**Interfaces:**
- Consumes: `RunnerCapacitySnapshot` shape from Task 1 (field names identical).
- Produces: registry path `infra/fleet-capacity.json`, contract path above.

- [ ] **Step 1: Write both files** with the exact schema string `cynic.fleet.capacity.v1`.
- [ ] **Step 2: Validate** Run: `jq empty infra/fleet-capacity.json && jq empty contracts/fleet-capacity.v1.json`. Expected: exit 0.
- [ ] **Step 3: Commit** both files together.

### Task 3: Scheduled observe arm `cynic fleet observe` (read-only)

**Files:**
- Create: `crates/cynic-cli/src/fleet_surface.rs` (`FleetCommand` with `Observe { json: bool }`, `execute_observe` reusing `SshBuildRunner::probe_free_ram_bytes`, disk probe, `parse_state_root`)
- Modify: `crates/cynic-cli/src/command_manifest.rs` (add `cynic fleet observe [--json]`)
- Modify: CLI command tree wiring wherever `fleet` family is declared (find `external_audit` family registration pattern and mirror)
- Test: in-file unit test on the snapshot-to-registry write helper (pure part extracted as `fn upsert_snapshot(registry: &mut Value, snap: &RunnerCapacitySnapshot)`)

**Interfaces:**
- Consumes: `RunnerCapacitySnapshot`, `decide_runner_usability` (Task 1); runner list from `infra/registry.json`.
- Produces: `fn upsert_snapshot(registry: &mut serde_json::Value, snap: &RunnerCapacitySnapshot)`; CLI arm `cynic fleet observe [--json]`.

- [ ] **Step 1: Write failing test** `upsert_replaces_stale_snapshot_same_runner` (two snapshots same runner_id, second wins).
- [ ] **Step 2: Run to verify it fails** Run: builder `test` phase `--package cynic-cli --filter fleet_observe`. Expected: compile fail.
- [ ] **Step 3: Implement** the arm + helper. `observe` writes `infra/fleet-capacity.json`, prints per-runner usability, exits non-zero when any runner is muted/degraded.
- [ ] **Step 4: Run tests to verify they pass** Run: same proof with `fleet_observe` filter (count > 0) plus `check` phase for the whole `cynic-cli` package.
- [ ] **Step 5: Commit** with proof SHAs.

### Task 4: Placement reads the registry (no cold probe)

**Files:**
- Modify: `crates/cynic-build/src/place.rs` (`place_build` gains a `snapshots: &[RunnerCapacitySnapshot]` parameter; cold-probe branches removed; unknown runner → `capable: false` with reason `unobserved: run fleet observe`)
- Test: existing in-file tests in `place.rs` updated to pass snapshots (`unmeasurable_is_incapable_never_zero` stays green by construction: `None` fields → muted)

**Interfaces:**
- Consumes: `RunnerCapacitySnapshot`, `decide_runner_usability` (Task 1).
- Produces: `pub fn place_build(task_class, floor_ram_bytes, snapshots, now) -> PlacementDecision` (exact new signature fixed here; Task 5 caller adapts).

- [ ] **Step 1: Update tests** to the new signature, including `unknown_runner_names_observe_command` asserting the reason contains `fleet observe`.
- [ ] **Step 2: Run to verify they fail** Run: builder `test` phase `--package cynic-build --filter place`. Expected: compile fail (signature changed).
- [ ] **Step 3: Implement** the new `place_build`; delete cold-probe code paths.
- [ ] **Step 4: Run tests to verify they pass** Run: same proof, all place tests green.
- [ ] **Step 5: Commit** with proof SHA.

### Task 5: Wire the caller + alert on muted

**Files:**
- Modify: `crates/cynic-node/src/ports/build_runner_adapter.rs` (trigger path loads `infra/fleet-capacity.json`, passes snapshots to `place_build`)
- Modify: `crates/cynic-network/src/watchdog_alerter.rs` (or new small fn) to map muted/degraded runners to `NotificationOutbox` entries
- Test: adapter-level test with a fixture registry containing one muted runner → placement refuses with the observe message; alerter test asserts one outbox entry per muted runner

**Interfaces:**
- Consumes: `place_build` new signature (Task 4), `NotificationOutbox` entry shape (read from existing code, do not invent).
- Produces: wired trigger path; alert entries.

- [ ] **Step 1: Write failing tests** for both behaviors.
- [ ] **Step 2: Run to verify they fail.**
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run tests to verify they pass** (builder `test` for `cynic-node` filter + `cynic-network` filter).
- [ ] **Step 5: Commit** with proof SHAs.

### Task 6: Signed breakglass gesture + operator runbook (Seat: controller for live steps)

**Files:**
- Modify: `crates/cynic-cli/src/fleet_surface.rs` (add `Remediate { runner: String, action: String, yes: bool }`; allowed actions: `gc-disk`, `restart-probe`; bounded argv only, executed via existing SSH port)
- Test: unit test `remediate_refuses_without_yes`, `remediate_refuses_unknown_action`.

**Interfaces:**
- Consumes: existing SSH exec port (read its exact signature from code, do not invent).
- Produces: `cynic fleet remediate <runner> --action <gc-disk|restart-probe> --yes`.

- [ ] **Step 1-4: Same TDD cycle** (builder proofs, implementer seat).
- [ ] **Step 5: Controller live run** (Seat: controller — the operator, not the implementer): run `cynic fleet observe` against the real fleet, paste the per-runner usability lines into the PR. The implementer reaches DONE without this step; the PR does not merge without it.
- [ ] **Step 6: Commit** code; open PR; controller merges after the live run.

### Task 7 (deferred, phase B): Mesh-pushed snapshots

Not implemented in this plan. When phase A is live, replace scheduled SSH probing with runner-pushed snapshots over the QUIC mesh (`fleet_membership` admission + `telemetry` shape). This task only records the seam: `execute_observe` must keep working unchanged when its snapshot source changes from probe to mesh inbox.

---

## Self-Review

- Spec coverage: design points 1–5 map to Tasks 1–6; phase B recorded as Task 7.
- Step scan: each step names one action, one command, one expected output.
- Type consistency: `RunnerCapacitySnapshot`, `RunnerUsability`, `decide_runner_usability`, `upsert_snapshot`, `place_build` signatures fixed once and reused verbatim.
- Review Focus: five lines above, each pinned to its owning task.
- Proportion: plan states interfaces and tests, not bodies.
