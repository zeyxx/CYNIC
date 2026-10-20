# Supervision Gaps and Specs — Open Decisions With Acceptance Criteria

**Status:** DESIGN
**Schema:** `cynic.supervision_gaps.v1`
**Authority:** `contracts/supervisor-contract.json` stays normative. This document records what the S1–S5 slices deliberately left open, the decision each gap needs, and what `go` will execute. It authorises no code.
**Depends on:** `docs/design/supervision-convergence-map.md`

---

## How to read this document

Each gap carries: measured state (file:line, never prose), why the slice stopped there, numbered decision options with a recommendation, and the owning future slice. A gap closes only by an operator `go` on its option, followed by an edge-closed implementation slice with builder proof.

---

## G1. Inference 24/7 custody (S3b)

**Measured:** `crates/cynic-inference/src/adapters/local/process_supervisor.rs:449-456` spawns then `drop(child)`; pidfile is the post-exit proof of life. `cynic-supervisor::spawn_backend` returns a `KillOnDrop` handle: the child dies with the holder.

**Why open:** converging spawn changes who the child outlives. No living parent exists yet on the process path (`systemd_user.rs` is a separate opt-in backend).

**Options:**
1. (R) Supervisor daemon holds inference handles; CLI `start` becomes request-to-supervise, pidfiles burn after proof. Owner: S3b slice, after `cynic supervise` parents one role.
2. Keep detach forever; shared spawn covers node only, inference keeps its own spawn with the shared stop ladder. Owner: record as declared divergence.
3. Migrate process users to `systemd_user` and burn the pidfile backend. Owner: inference lifecycle decision, needs linger/user-bus proof on every target host.

**Acceptance:** one named holder per running `llama-server`; kill-holder-kills-child proven; CLI exit no longer load-bearing; `installed_runtime` green throughout.

## G2. Restart timing divergence

**Measured:** node `config/defaults.rs:13-27` = `5 attempts / 2s / 120s / 10s`. Contract = `on-failure / 5s / 60s / 10s`, no `max_attempts`. S1/S5 share the formula only; values untouched.

**Why open:** changing `2→5` / `120→60` alters production restart timing. That is a policy decision, not a refactor.

**Options:**
1. (R) Adopt contract `5/60`, keep node `max_attempts 5` as the declared bound the contract lacks; record the bound addition as a contract amendment. Owner: S5b slice (config defaults + node tests + observed restart drill).
2. Amend the contract to node values `2/120`. Owner: contract amendment + S1 constant update.
3. Per-role policy table in the contract (node `2/120`, inference `5/60`). Owner: contract amendment; only with two proven consumers needing different values.

**Acceptance:** one declared table; node defaults equal it byte-for-byte; `decide_restart` drill test pins `5,10,20,40,60` on contract policy.

## G3. Health-to-lifecycle mapping

**Measured:** unit health is 3-valued (`active/degraded/unknown`, `decision.rs:112-117`); lifecycle is 7-valued (`SupervisorState`, S1). No mapping exists, by design.

**Why open:** `degraded → draining` vs `→ failed` is semantics no slice may invent.

**Options:**
1. (R) `degraded → failed` for restart decisions (it already blocks convergence today), `unknown → unknown`; `draining` reserved for commanded stops only. Owner: S4 slice with the supervised parent.
2. Keep both vocabularies separate permanently (health vs lifecycle as distinct types). Owner: record as architecture decision; S4 closes as observe-only.
3. `degraded → draining` with auto-recover. Owner: needs drain/recover proof first; premature today.

**Acceptance:** mapping function is total (no `_` arm hides a case), tested over all 3 inputs, decision outputs byte-identical to today.

## G4. `cynic supervise observe` surface

**Measured:** contract phase 3. No CLI surface exists. Observation adapters exist (`service_supervisor_observation.rs`, `process_convergence/`, inference statuses).

**Why open:** new operator surface; grammar ownership is the CLI's, behind the thin-surface law (nest under an owning family, no hyphenated top-level).

**Options:**
1. (R) `cynic supervise observe [--role <id>]` read-only first; JSON observation schema `cynic.supervisor_observation.v1`; mutation arms (`start/stop`) arrive only with `--apply`-style bounds. Owner: S6 slice (CLI surface + protocol observation types).
2. Observation via existing `node observe` / `operator_status` only; no new surface. Owner: document and close.

**Acceptance:** observe runs with zero mutation (provable: no spawn/signal code path reachable); unknown stays unknown; output validates against the observation schema.

## G5. `process_observation.rs` placement — CLOSED: KEEP in host

**Measured:** `crates/cynic-host/src/process_observation.rs`, 394 LOC. Closure proof (2026-10-15):
outgoing = `crate::runtime_layout` (host-internal) + `std::process::Command` +
`tokio::process` + serde/std — not std+protocol only. Incoming = 10 host
modules (`service_supervisor_observation`, `storage_capabilities/local`,
`runtime_layout`, `runtime_network_preflight`, `endpoint_observation`,
`adapters/{local_disk,local_build_artefact,principal_manager,local_observation}`,
`lib.rs`).

**Decision:** KEEP. It is a generic command-capture utility
(`capture_command`, `capture_argv`, `command_json`, `command_stdout`,
`reexec_current_process_with_env`), shared across storage, network, and
observation chains — not supervision-specific. Moving it to
`cynic-supervisor` would drag host `runtime_layout` and tokio process
spawning into the L1 effects crate and strand 10 importers. Perimeter:
host-owned process-execution helper; supervision uses the typed
`cynic-supervisor` spawn/stop ladder for CYNIC-owned children instead.

## G6. Kernel and MCP role migration

**Measured:** contract declares `kernel` (`daemon kernel`, boot-sensitive) and `mcp` (`mcp serve`, projection boundary `mcp_is_projection_not_bootstrap`, `foundation.json:65,175-177`).

**Why open:** node must prove the path first (map phase order).

**Options:**
1. (R) Kernel second (host supervisor stays outer during boot, CYNIC observes then parents), MCP last with spawn-only integration (never a Cargo edge). Owner: S7/S8 slices post-node-proof.
2. MCP before kernel. Owner: rejected ordering — projection boundary needs the longest soak, not the shortest.

**Acceptance:** per-role proof (start/stop/restart/backoff/read-back evidence); `cargo metadata` proves zero manifest edge to `cynic-mcp`.

## G7. `spawn_with_retries` hardcoded retry

**Measured:** `supervise.rs:spawn_with_retries` sleeps fixed `1s`; call site `runtime_supervision.rs:229-237` passes `max 3`. Protocol now owns `decide_restart` (S1) but nobody calls it.

**Why open:** replacing fixed `1s` with backoff changes spawn-storm behavior; needs its own drill.

**Options:**
1. (R) Route spawn retries through `decide_restart` with the role policy; burn the fixed `1s`. Owner: S5b with G2 (same timing-policy decision).
2. Keep fixed retry for spawn (admission-time) and backoff for restarts (runtime) as two declared policies. Owner: record as declared divergence with rationale.

**Acceptance:** spawn-storm drill (backend refusing connections) shows bounded attempts with the declared delays; no unbounded loop.

## G8. Pre-existing node lib break (foreign)

**Measured:** `crates/cynic-node/src/usecases/build_scheduler/mod.rs:618` defines `settlement_proof_evidence_ref`; all callers (`:785,:925,:994`) sit inside `mod tests` (`:667`). `cargo check -p cynic-node --lib` fails `deny(dead_code)`. File outside every supervision diff; T2 scheduler work owns it.

**Why open:** foreign in-flight work; touching it collides.

**Options:**
1. (R) Owner bounds the function (and its import) with `#[cfg(test)]` per the documented test-only-caller pattern.
2. Owner restores a non-test caller.

**Acceptance:** `cargo check -p cynic-node --lib` green on a clean tree; supervision proofs re-run unchanged after.

## G9. `SupervisorState::parse` (deferred S4a')

**Measured:** `as_str` exists (S1); no `parse`. No consumer needs it yet.

**Why open:** value-free until the parent or CLI derives state from strings.

**Options:**
1. (R) Add with G4 (first real consumer); total over the 7 contract names, roundtrip-tested.
2. Add now speculatively. Owner: rejected — no consumer, no proof of need.

## G10. Inference/CLI state derivation

**Measured:** `LocalRuntimeState` (5 values, `inference/domain/local_runtime.rs:47`), `BackendStatus` (5 values), CLI `actively_supervised` (`operator_status.rs:98`) derive from inference vocabulary, never from `SupervisorState`.

**Why open:** derivation direction (inference maps to generic, or CLI maps both) is a layering decision needing G3's mapping first.

**Options:**
1. (R) Inference implements `supervisor_state()` (it already depends on protocol); CLI derives from the generic. Owner: post-G3 slice.
2. CLI maps both vocabularies itself. Owner: rejected — decision logic would live in the presentation layer.

---

## Execution order (recommended)

G8 (foreign, unblocks full proofs) → soak 30 days → standalone/detach
removals (one `go` per step) → host e2e drills on the 24/7 machine.

Landed 2026-10-15: G2+G7 (S5b), G4+G9 (S6 + parent step 5 `stop`),
G3+G10 (S7), G5 (KEEP), G1-inference + G6 (parent steps 1–4).
Each arrow is an operator `go`. No slice starts without one.
