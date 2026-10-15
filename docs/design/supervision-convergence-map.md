# Supervision Convergence Map — Domain and Causal-Chain Extraction Plan

**Status:** DESIGN
**Schema:** `cynic.supervision_convergence_map.v1`
**Authority:** `contracts/supervisor-contract.json` is normative. `contracts/foundation.json` and `contracts/pruning-policy.json` take precedence. This document has zero execution authority; it is the human-facing extraction map for the supervisor convergence.
**Depends on:** `docs/design/sovereign-supervisor.md`, `docs/design/single-binary-capability-selected.md`

---

## 0. Decision

Use the sovereign supervisor contract as the convergence spine for the de-monolithisation chantier. Extract supervision by domain and causal chain into target crates and roles, reduce code duplication and unnecessary algorithmic complexity, and avoid creating a new monolith inside `cynic supervise`.

This document authorises no code move. It records measured state, duplication groups, target domains, burn candidates, law pins, and edge-closed slices. Each slice requires its own operator approval and builder proof.

---

## 1. Measured starting state (2026-10-14)

All line counts measured with `wc -l` on the working tree. No code changed by this document.

### 1.1 Crate totals

| Crate | Total `src/**/*.rs` LOC |
|---|---|
| `cynic-host` | 84 606 |
| `cynic-node` | 55 941 |
| `cynic-protocol` | 36 895 |
| `cynic-cli` | 32 903 |
| `cynic-inference` | 13 896 |

`cynic-host` is the god object. It holds 80+ top-level modules. Supervision must leave `cynic-host`, never enter it temporarily.

### 1.2 Supervision fragments by file

| # | File | LOC | Role in causal chain |
|---|---|---|---|
| 1 | `crates/cynic-node/src/supervise.rs` | 357 | spawn + graceful stop + backoff, `process-wrap` process-group / JobObject |
| 2 | `crates/cynic-node/src/runtime_supervision.rs` | 238 | node watch loop, owns `NodeState`, delegates to `lifecycle` |
| 3 | `crates/cynic-node/src/runtime_supervision/lifecycle.rs` | 317 | lifecycle detail, `ExitReason` handling |
| 4 | `crates/cynic-node/src/domain/supervision.rs` | 15 | `ExitReason` enum, pure |
| 5 | `crates/cynic-protocol/src/supervision.rs` | 113 | `ServiceSupervisorObservation`, pure status aggregation |
| 6 | `crates/cynic-host/src/service_supervisor_observation.rs` | 482 | systemd / openrc / process-port observation adapter |
| 7 | `crates/cynic-host/src/process_convergence/` | 1 004 | decision 216 + mod 35 + observer 22 + port 13 + query 279 + record 367 + types 72 |
| 8 | `crates/cynic-host/src/resilient_runner.rs` | 384 | circuit breaker + retry + backoff over `ArgvRunner`, transport chain |
| 9 | `crates/cynic-host/src/domain/process/` | 352 | mod 10 + probe 78 + types 264, restart policy |
| 10 | `crates/cynic-inference/src/adapters/local/process_supervisor.rs` | 590 | pidfile + `kill` shell + `/proc` + `curl` readiness + direct `Command` spawn |
| 11 | `crates/cynic-inference/src/adapters/local/docker_supervisor.rs` | 301 | ODS `.env` + `docker ps` + `dream-cli restart` adapter |
| 12 | `crates/cynic-inference/src/adapters/local/systemd_user.rs` | 464 | user systemd adapter, part of local runtime selection |
| 13 | `crates/cynic-host/src/process_observation.rs` | 394 | process observation surface |
| 14 | `crates/cynic-host/src/tmux_session_supervision.rs` | 530 | tmux session supervision, not process supervision |
| 15 | `crates/cynic-host/src/tunnel_supervisor.rs` | 122 | tunnel supervision, not process supervision |

Local inference adapters total (`crates/cynic-inference/src/adapters/local/`): 2 558 LOC across 10 files.

### 1.3 Already-decided authorities

- `contracts/supervisor-contract.json`: normative supervisor policy. Roles `node`, `kernel`, `mcp`. Foreground only. Pure decision core. OS-gated imperative shell. Read-back proof. Portability by `cfg` boundary.
- `contracts/foundation.json`: `mcp_is_projection_not_bootstrap`. No crate manifest may depend on `cynic-mcp`. Measured note: `mcp_is_not_a_dependency_of_anything`.
- `crates/cynic-node/tests/architecture_boundary.rs:35-101`: `main.rs` must delegate to `runtime_supervision::run_node`; `runtime_supervision.rs` must stay high-level and delegate to `runtime_supervision/lifecycle.rs`; lifecycle must not own federation detail. This is a law pin. Moving `runtime_supervision` without amending this test breaks existing law silently.

---

## 2. Causal chain (the extraction spine)

The chain is the already-documented CYNIC coherence pattern from `resilient_runner.rs:1-8`:

```text
declared role (contract)
  -> pure decision (admit / restart / drain / observe)
    -> bounded OS effect (spawn / signal / process-group / health probe)
      -> read-back observation (process / service-manager / health)
        -> convergence record (epoch + evidence digests)
          -> role state
```

Every fragment in section 1.2 is classified against this chain in section 3. Fragments that do not belong to this chain are explicitly marked DO-NOT-MOVE.

---

## 3. Duplication groups (measured, not assumed)

### D1. Backoff — three policies, two chains

- `crates/cynic-node/src/supervise.rs:27-110`: `Backoff` struct, `attempt`, `max_attempts`, `initial_delay`, `max_delay`, `min_uptime`, `next_delay = initial * 2^(attempt-1)` capped, `reset_if_stable`, `wait_or_shutdown` with `CancellationToken`. Chain: process supervision.
- `crates/cynic-host/src/resilient_runner.rs:49-75,177-200`: `RetryPolicy` with `max_attempts`, `initial_backoff_ms`, `backoff_multiplier`, `max_backoff_ms`, plus circuit breaker `Closed/Open/HalfOpen`, `failure_threshold`, `cooldown_ms`. Chain: transport argv runner.
- `crates/cynic-host/src/domain/process/types.rs:47`: restart comment referencing systemd `RestartPreventExitStatus` semantics (`> 3 restarts within 5 minutes -> backoff`). Chain: service policy declaration.
- `crates/cynic-intel/src/adapters/helius/rpc.rs:154`: `Duration::from_secs(1 << attempt)` inline retry. Chain: external RPC client. Out of supervisor scope.

Verdict: D1 is two convergent supervisor backoffs (`node::supervise::Backoff` + host process restart policy) plus one transport retry policy that must NOT be merged into supervision. Merge only the supervision chain. `resilient_runner` circuit breaker stays in transport.

### D2. Stop / kill ladder — strong vs weak

- Strong: `crates/cynic-node/src/supervise.rs:165-190`: `graceful_stop`, Unix `libc::SIGTERM` on process group, Windows `start_kill`, bounded `timeout`, then `start_kill` + `wait`. Tested at `supervise.rs:338-347`.
- Weak: `crates/cynic-inference/src/adapters/local/process_supervisor.rs:86-96`: `kill_pid`, shell `Command::new("kill")` with `-TERM`, `thread::sleep(400ms)`, `/proc/{pid}` check, then `-KILL`. Callers at lines 416, 422, 477, 540.

Verdict: MERGE weak into strong. The shell `kill` path violates zero-shell invariant and `/proc` polling duplicates `wait`. Converged in S3a via direct `rustix` signal delivery (workspace `unsafe_code = "forbid"` rules out raw `libc::kill`); the detached pidfile lifecycle itself is preserved until the supervised-parent decision lands (S3b, deferred).

### D3. Spawn — three substrates, one contract

- `crates/cynic-node/src/supervise.rs:122-156`: `spawn_backend`, `CommandWrap`, Unix `ProcessGroup::leader + KillOnDrop`, Windows `JobObject + KillOnDrop`, inherited stdio, `working_dir`, `env`. Reference implementation, extracted to `cynic-supervisor` in S2.
- `crates/cynic-inference/src/adapters/local/process_supervisor.rs:437`: `Command::new(&bin)` direct spawn, pidfile write, log file, then `drop(child)` detach. No process group, no `KillOnDrop`, orphan risk by design (CLI exits, server stays).
- `crates/cynic-inference/src/adapters/local/docker_supervisor.rs:75,207,262`: `docker ps` probe + `dream-cli restart llama-server`. Not a process spawner; container-runtime adapter.

Verdict: S2 converges node spawn onto the shared effects crate. Inference detach keeps its own spawn until S3b decides who holds the child handle 24/7 (a `KillOnDrop` handle would die with the CLI). Keep docker adapter as a separate runtime port behind the same decision core.

### D4. Observation / read-back — four surfaces

- `crates/cynic-protocol/src/supervision.rs:7-72`: pure `ServiceSupervisorObservation` + `service_supervisor_status` (active / degraded / unknown, unknown never coerced).
- `crates/cynic-host/src/service_supervisor_observation.rs` (482 LOC): systemd / openrc / process-port provider adapter + aggregation.
- `crates/cynic-host/src/process_convergence/` (1 004 LOC): decision, observer, query, record, `CausalConvergenceEpoch`, evidence digests, residual unknowns.
- `crates/cynic-cli/src/operator_status.rs:36-118`: `actively_supervised` derived from local runtime read-back, routing config stays declared state.
- `crates/cynic-host/src/process_observation.rs` (394 LOC): adjacent process observation surface, convergence candidate.

Verdict: MERGE along the chain in S4. Pure status stays in protocol. Provider adapters stay in host. Epoch/record stays in convergence. CLI derives, never decides.

### D5. Restart policy declaration — two homes

- `crates/cynic-node/src/config::RestartConfig` (consumed by `supervise::Backoff::from_config` at `runtime_supervision.rs:73`): `max_attempts 5`, `initial 2s`, `max 120s`, `min_uptime 10s` (`config/defaults.rs:13-27`).
- Contract `lifecycle.restart.backoff`: `initial 5s`, `max 60s`, `min_uptime 10s`, no `max_attempts`.
- `crates/cynic-host/src/domain/process/types.rs` (264 LOC service policy): crash-loop `threshold 3 / window 300s`.

Verdict: S1 declares the canonical policy in protocol (`5/60/10`, bound caller-supplied). Node `2/120/5` is a recorded divergence, migrated in S5, not silently overwritten here.

---

## 4. Target domains by causal chain

| Chain link | Target owner | Responsibility | Receives from |
|---|---|---|---|
| Declared role | `contracts/supervisor-contract.json` + `cynic-protocol::supervision` | role id, argv, restart, signals, health, privilege, observation schema | nothing; this is law |
| Pure decision | `cynic-protocol::supervision` (S1 done) | `RestartPolicy`, `BackoffState`, `decide_restart`, `SupervisorState`; zero I/O | contract |
| Bounded OS effect | `cynic-supervisor` (S2 done) | `spawn_backend`, `graceful_stop`, process-group / JobObject, kill ladder, `cfg(unix)` / `cfg(windows)` gates | protocol pure types only |
| Service-manager read-back | `cynic-host::service_supervisor_observation` (adapter, stays) | systemd / openrc / process-port providers, aggregation mapped onto protocol status | protocol types |
| Node role | `cynic-node::runtime_supervision` (consumer, shrinks) | node watch loop, federation, config reload; calls shared core for spawn / stop | shared core |
| Inference roles | `cynic-inference::adapters::local` (S3a zero-shell, S3b deferred) | process / docker / systemd-user adapters behind `LocalInferenceRuntimePort` | shared ladder |
| Convergence record | `cynic-host::process_convergence` + `cynic-eval::convergence` | epoch, evidence envelope, digests, residual unknowns | observation |
| Operator view | `cynic-cli` supervise surface (future) | `observe` first, bounded `start/stop` later; derives from observation, never invents state | convergence |

Rules:

- One crate, one domain, one source of truth (atomic domain fusion).
- Pure types: 100% pure, no `std::process`, no `tokio`, no sockets, no syscalls.
- Ports: traits only, zero implementation.
- `lib.rs`: re-exports only.
- No `cynic-host` as temporary destination. Ever.
- No new dependency from any crate to `cynic-mcp`. Spawn of the existing projection binary is permitted; a Cargo edge is not.
- Wire schema strings (`cynic.service_supervisor_observation.v1`, `cynic.host.process_convergence_decision.v1`, `cynic.host.process_convergence_record.v1`, `cynic.supervisor_observation.v1`) stay byte-identical across moves.

---

## 5. Do not move (explicit non-scope)

- `crates/cynic-host/src/resilient_runner.rs` (384 LOC): transport circuit breaker. Different chain, different failure semantics. Reference, do not absorb.
- `crates/cynic-host/src/tmux_session_supervision.rs` (530 LOC): tmux session management. Adjacent, not process supervision.
- `crates/cynic-host/src/tunnel_supervisor.rs` (122 LOC): tunnel lifecycle. Adjacent, not process supervision.
- `crates/cynic-intel/src/adapters/helius/rpc.rs:154`: external RPC retry. Out of scope.
- `crates/cynic-inference/src/adapters/local/docker_supervisor.rs` internals (ODS `.env`, `dream-cli`): container runtime adapter. Converge its decision interface, not its container mechanics.
- Windows / macOS adapters: architecture keeps `cfg` gates open; no adapter built without a real user on that host.

---

## 6. Complexity to burn, not extract

- Shell `kill` in `process_supervisor.rs:86-96`: BURNED in S3a (`rustix` direct delivery, same 400ms ladder). No preservation shim.
- `curl`-via-shell readiness in `process_supervisor.rs:209-245`: BURNED in S3a (`reqwest::blocking`, 2s timeout, `"000"` unreachable code preserved). One request replaces two.
- `/proc` polling + pidfile staleness handling: KEPT until S3b. Pidfiles are the detached lifecycle's proof of life, not yet replaceable without a living parent.
- Duplicate `Backoff` vs `RetryPolicy` confusion: resolved by naming. Supervisor backoff lives in protocol. Transport retry lives in transport. No shared `BackoffUtils` premature abstraction.
- `tmux_session_supervision` + `tunnel_supervisor` + process supervision sharing helpers by copy: if a helper is truly shared, it moves once into the destination with `pub(crate)` visibility and is imported back. Never duplicated both sides.

---

## 7. Law pins and vetoes (check before every slice)

1. `crates/cynic-node/tests/architecture_boundary.rs:35-101` pins `runtime_supervision::run_node` delegation and lifecycle placement, plus `pub enum ExitReason` in `domain/supervision.rs:80-82`. S1/S2/S3a touch none of the asserted symbols: green without amendment.
2. `contracts/foundation.json:65,175-177` pins `mcp_is_projection_not_bootstrap`. No slice adds a manifest edge to `cynic-mcp`; verify with `cargo metadata` per slice.
3. FCIS: `domain/` stays pure. `grep -rn "std::fs|std::process|Command::new" <dest>/src/domain` must be empty per slice.
4. Layer rule: the supervisor effects crate (L1) must not depend on `cynic-host`, `cynic-node`, or `cynic-cli`. Verify with `grep cynic-host|cli|node <dest>/Cargo.toml`.
5. Schema freeze: `SERVICE_SUPERVISOR_OBSERVATION_SCHEMA`, `PROCESS_CONVERGENCE_DECISION_SCHEMA`, `PROCESS_CONVERGENCE_RECORD_SCHEMA`, `SUPERVISOR_OBSERVATION_SCHEMA` byte-identical.
6. `unsafe_code = "forbid"` (workspace lints): raw `libc::kill` is unusable; `rustix` safe wrappers are the allowed path.

---

## 8. Slices (edge-closed, one proof each)

Each slice is small enough to review, moves an edge-closed set, re-routes every incoming and outgoing edge inside the same slice, and carries one builder proof per affected crate. A slice leaving a dangling import is rejected, not deferred.

### Slice S0 — Map (this document, no code)

Proof: this file exists, JSON contract untouched, no Rust changed.

### Slice S1 — Pure supervision vocabulary in protocol (done)

Added: `SupervisorState` (7 contract states), `RestartPolicy::from_contract`, `BackoffState`, `delay_for_attempt`, `decide_restart`, constants `5/60/10/15`. `ExitReason` stays in node (node-specific, single consumer). Proof: `cargo test -p cynic-protocol --lib supervision` 6/6.

### Slice S2 — Effects crate creation (done)

Created `cynic-supervisor` (`spawn_backend` + `graceful_stop`, own proof tests 3/3). Node `supervise.rs` delegates; `Backoff`, `spawn_with_retries`, call sites untouched. Proof: supervisor 3/3, node `supervise` 8/8, node boundary 18/18.

### Slice S3a — Inference zero-shell (done)

`kill_pid`: `rustix::process` direct delivery, same TERM/400ms/KILL ladder, `cfg(unix)` gated. `fetch_models_for_wait`: `reqwest::blocking`, 2s timeout, `"000"` unreachable preserved, one request replaces two. Detached pidfile lifecycle untouched. Proof: inference check + local-runtime tests + `installed_runtime` CLI test.

### Slice S3b — Inference supervised lifecycle (deferred)

Replace detach+pidfile with a held child handle. Requires a living parent (supervisor daemon or systemd user) and the `parent_scope` decision. Not executable until S2's parent exists and proves handle custody 24/7.

### Slice S4 — Observation convergence

Map `service_supervisor_observation` (482 LOC adapter) outputs + `process_convergence` (1 004 LOC) records onto protocol status vocabulary. No adapter moved into protocol. No epoch logic moved into the effects crate. CLI `operator_status.rs actively_supervised` derived from the same vocabulary.

### Slice S5 — Node loop shrink (includes S2-elargi)

`runtime_supervision.rs` + `lifecycle.rs` consume protocol `BackoffState`/`decide_restart` + supervisor effects only. Federation, config watch, kernel announce stay in node. Resolves the `2/120/5` vs `5/60/contract` divergence with its own evidence slice. File sizes must decrease; a slice that does not shrink node is the wrong slice.

### Explicitly deferred

- `cynic supervise` CLI surface: after S1-S2 proven. Observe first, mutation later.
- `kernel` and `mcp` roles: after `node` proven.
- `tmux` / `tunnel` supervision: separate maps, separate decisions.
- S2-elargi timing: bundled into S5, never into S2 (effects move and behavior change stay separate slices).

---

## 9. OS / hardware agnosticism (build only what use justifies)

Architecture is portable; implementation is per-host and justified:

- Allowed gates: `cfg(unix)`, `cfg(target_os = "linux")`, `cfg(windows)`.
- `cynic-supervisor` keeps the `ProcessGroup` vs `JobObject` split; inference `kill_pid` is `cfg(unix)`-gated with no shell fallback (pidfile liveness was already unix-bound via `/proc`).
- Forbidden in pure core (`cynic-protocol::supervision`): `std::process::Command`, `tokio::spawn`, sockets, syscalls, service-manager commands. Verified per slice by grep.
- No Windows / macOS adapter built now. No Linux-only assumption baked into pure types (no `/proc`, no pidfile, no systemd unit name in protocol).

---

## 10. Open decisions (carried, not resolved here)

1. `parent_scope` (from supervisor contract): spawn existing daemon roles first, then absorb internal supervision after proof. This map implements that order (S1-S5 before any parent absorber).
2. `host_supervisor_relationship`: systemd remains outer supervisor; CYNIC owns CYNIC-owned children and observations.
3. S3b custody: who holds the inference child handle 24/7. Decided when the supervised parent exists.
4. `process_observation.rs` (394 LOC) placement: convergence candidate, not yet classified. Classify at S4 with import-closure proof before moving.

---

## 11. Worktree isolation (learned 2026-10-15)

S1/S2/S3a were first implemented on the shared checkout `agent/hermes/build-latency-science` and wiped by a concurrent reset: tracked edits reverted, untracked crate/docs deleted. Only a surviving `git diff` fragment allowed reconstruction. All supervision work now lives on `agent/hermes/supervision-convergence` in a private worktree. Shared-tree discipline: never hold uncommitted supervision work across turns on a contested branch; commit early with agent identity.

---

## 12. Verification checklist per slice

- [ ] Cartography covers all crates touching the slice (host, kernel, protocol, inference, node, cli).
- [ ] Incoming + outgoing edges listed; tests consuming moved symbols move with the slice.
- [ ] Wire schema strings byte-identical.
- [ ] Destination self-imports rewritten (`use <dest>::` -> `crate::` after `cp`).
- [ ] No `git add -A`; explicit paths only; `git diff --cached --name-only` reviewed.
- [ ] `cargo check -p <dest>`, `cargo check -p <src>`, plus every dependent crate.
- [ ] Boundary tests run where the slice intersects them; law change has operator release.
- [ ] `Cargo.lock` regenerated via `cargo metadata` if a manifest edge changed, never hand-edited.
- [ ] Builder proof per affected crate (canonical gate, not local-only claim).
- [ ] Work committed with agent identity before any context switch.

---

## 13. Next step

### Landed after this map (2026-10-15, branch `agent/hermes/supervision-convergence`)

- S5 done: `Backoff::next_delay` delegates to protocol `delay_for_attempt`.
- S5b done (G2+G7 closed): node defaults `2/120` → contract `5/60` (`max_attempts 5`
  kept as the declared bound); `spawn_with_retries` via `decide_restart`.
- S6 done (G4+G9): `cynic supervise observe [--role] [--status-file]`,
  `SupervisorState::parse`, `SupervisorObservation`, `SUPERVISED_ROLE_IDS`.
- S7 done (G3+G10): `unit_status_state` mapping consumed by host convergence
  (byte-identical outputs); `supervisor_state()` on `LocalRuntimeState`,
  `BackendStatus`, and CLI derivation.
- G5 closed: `process_observation.rs` KEEP in host (closure proof in gaps doc).
- Parent steps 1–5 done (G1 inference closed, G6 closed): `parent.rs` run loop
  (lock, restarts, status file), `supervise run`, wired `observe`, 4 systemd
  units (node/kernel/inference/mcp), inference refuse-when-parented,
  `supervise stop --apply` with read-back proof. No `start` arm: `run`
  foreground and the systemd units already start.
- S3b executes as parent step 2 (detach burns at the removal step, not before).
- S4-real executes as S6+S7 (observation vocabulary already converged; only
  the lifecycle mapping was missing).

### Still open (not executable now)

- G8 (foreign): `settlement_proof_evidence_ref` test-only caller breaks
  `cynic-node --lib`; CLI proofs (`supervise_surface`, taxonomy,
  `installed_runtime`) re-run after the owner's fix.
- Soak + removals: parent crash rate vs child crash rate over 30 days 24/7,
  then standalone/detach removal with its own `go` per step.
- `cynic supervise stop` e2e against a live parent on the 24/7 host
  (unit tests drill it; host drill pending deployment).
