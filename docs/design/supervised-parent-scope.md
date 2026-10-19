# Supervised Parent Scope — Design Spec

**Status:** SPEC (no code authorised)
**Schema:** `cynic.supervised_parent_scope.v1`
**Authority:** `contracts/supervisor-contract.json` normative. This spec resolves its `parent_scope` open decision and owns G1 (custody) + G6 (roles). Implementation needs a separate operator `go` per rollout step.
**Depends on:** `docs/design/supervision-gaps-and-specs.md` (G1–G10), S1–S7 slices.

---

## 1. Problem

Every S1–S7 slice converges decision math, vocabulary, and observation, but no living process holds a child handle. Custody facts today:

- Node: `runtime_supervision` holds the handle inside its own watch loop — the supervised process supervises itself.
- Inference: pidfile + detach — nobody holds the handle; `KillOnDrop` semantics impossible.
- `supervise observe` reports `Unknown` for all roles because there is no state holder to ask.

The parent is the missing state holder: one process per role that owns the `KillOnDrop` handle, the restart counter, the last exit reason, and answers observation.

## 2. Non-goals

- Not a service manager. systemd stays outer (boot, socket activation, logging to journal). The parent parents CYNIC-owned children only.
- Not an init (PID 1). No orphan adoption, no signal proxying for foreign trees.
- No mutation surface in step 1. `start/stop` arms arrive after `observe` proves read-back against the parent.
- No Windows/macOS parent now. Architecture keeps `cfg` gates; implementation lands on Linux hosts first (the only 24/7 CYNIC iron).

## 3. Custody model

One parent process per role: `cynic supervise run --role <id>`.

```text
systemd unit cynic-supervise-node.service (outer, boot, restart)
  └─ cynic supervise run --role node (parent, holds handle)
       └─ daemon node (child, KillOnDrop: dies with parent, never orphans)
```

Rules:

- The parent holds the child handle for its whole lifetime. Parent death kills the child (by handle, not by signal race). systemd restarts the parent, which restarts the child under declared policy. Total outage on parent crash is bounded by systemd `RestartSec` + child boot, and it is explicit — never a half-supervised child.
- Exactly one parent per role per host. Enforced by a lock file with owner PID + freshness (`/run/cynic/supervise-<role>.lock` on Linux; `cfg`-gated elsewhere). Second parent exits with a named error, never steals the child.
- The parent never detaches the child. Detach + pidfile (S3a inference backend) burns when the role migrates under the parent.
- Restart decisions come from `decide_restart` with the contract policy (S5b values). The parent owns `restart_count` and `last_exit_reason` in memory and exposes them to observation.

## 4. State ownership

| State | Owner | Survives parent restart? |
|---|---|---|
| Child handle | parent process | No — child dies with parent, both restart |
| `restart_count`, `last_exit_reason` | parent memory | No — reset is honest (new parent, new epoch) |
| Declared policy | contract + role config | Yes — files, not memory |
| Epoch continuity | convergence record (existing `process_convergence` chain) | Yes — records persist, parents come and go |
| Observation answers | parent, read at request time | N/A — observation is point-in-time |

`restart_count` resetting on parent restart is not data loss: the convergence record chain already persists epochs with evidence digests. The parent reports its own epoch; continuity lives in records, not in parent memory.

## 5. Observation transport (decision required)

`supervise observe` must read parent state. Options:

1. (R) Status file written atomically by the parent (`/run/cynic/supervise-<role>.status.json`, write-temp + rename, 1s cadence or on change). `observe` reads the file; absent file = `Unknown` + `supervisor_daemon_absent` (exactly today's output — S6 stays byte-identical when no parent runs). No sockets, no new IPC, no privilege dance beyond file perms. Stale-file risk bounded by a `observed_at` freshness check (>5s = `Unknown` + `supervisor_status_stale`).
2. Unix socket request/response. Live queries, but new attack surface, new protocol, new failure modes (backlog, perms, stale socket after crash). Unjustified before step 1 proves the file.
3. Parent prints on SIGUSR1 / CLI signals parent. Signal IPC is lossy and racy; rejected.

## 6. Failure modes (declared, not handled by hope)

- Parent crashes: child dies by handle, systemd restarts parent, child restarts under policy. Declared outage, explicit in records.
- Child refuses to die on SIGTERM: existing `graceful_stop` ladder (SIGTERM → bound → SIGKILL) already covers; parent just calls it.
- Child exits 0 (clean): parent does NOT restart (exit reason `clean`, matches node `ExitReason` semantics). Records the stop, stays parent-alive awaiting config change or `stop` arm.
- Lock held by dead PID: freshness check (PID liveness + mtime) lets the new parent take over; takeover is recorded as residual unknown on first observation, not silent.
- Disk-full `/run`: status writes fail; parent keeps supervising (supervision never depends on observability), observation degrades to stale/absent with explicit residuals.
- Binary upgrade: new binary restarts the parent unit → child restarts. No hot-swap. Downtime is declared per role (node restart is already a designed event).

## 7. Rollout order

1. `node` under parent on the 24/7 host, `observe` reading the status file. `daemon node` keeps working standalone (no flag day).
2. Inference `llama-server` under parent (S3b executes here: detach burns, handle held). CLI `start` becomes request-to-parent or stays direct with a deprecation record — decided at build time, not here.
3. `kernel` under parent (boot-sensitive: parent unit `After=` kernel prerequisites, never blocks boot; observation-first soak before custody).
4. `mcp` under parent last: spawn-only integration of the existing projection binary, `cargo metadata` proves zero manifest edge each step.
5. Mutation arms (`supervise start/stop`) only after 1–2 prove read-back.

Each step: separate `go`, separate builder proof, `daemon <role>` standalone keeps working until the step's own removal decision.

## 8. Acceptance per step

- One parent per role (second parent exits named, proven by drill).
- Kill-parent-kills-child proven (no orphans after SIGKILL of parent).
- `observe` output validates against `cynic.supervisor_observation.v1`; absent parent = today's `Unknown` output byte-identical.
- Restart drill: kill child 6 times → 5 restarts at `5,10,20,40,60` then `GiveUp` with `max_restart_attempts_reached` in `last_exit_reason`.
- Lock-takeover drill: kill -9 parent, new parent takes over, first observation carries the takeover residual.
- No new dependency from any crate to `cynic-mcp`; no hyphenated commands; FCIS grep clean on touched domains.

---

## 9. Adversarial review (against this spec)

Written after the spec, read as an attack. Each item either breaks the spec (marked BREAKS, needs a spec amendment) or is accepted as residual risk (marked ACCEPTED, needs a drill or a record).

### A1. KillOnDrop makes every parent crash a full outage — BREAKS the "supervision improves availability" story

The spec sells the parent as the availability answer, but handle custody means parent death = child death, where today a crashed watcher can leave a healthy child running. If the parent is buggier than the child (new code always is), availability gets worse, not better.

**Answer:** true, and the spec must not oversell. The parent's value is not fewer outages — it is explicit outages with bounded recovery instead of silent half-supervision. Amendment: rollout step 1 keeps `daemon node` standalone as fallback for one soak period; the parent becomes the default only after its own crash rate is measured below the child's. Add a spec rule: no step removes the standalone path until the parent proves a lower crash rate over 30 days of 24/7 operation.

### A2. Status file cache: `observe` reads a file, not the parent — BREAKS liveness claims

A 1s-cadence file means observation is always stale by design, and a wedged parent (alive but not writing) looks identical to a dead parent only after the 5s freshness trip. Worse: an operator reading the file during the 5s window sees a healthy state for a dead supervision loop.

**Answer:** partially true. The freshness residual (`supervisor_status_stale`) is the honest signal, and S6 already reports `Unknown` when nothing holds state. But the spec under-specifies the wedged-parent case. Amendment: the status file carries a monotonic `heartbeat_counter` plus `observed_at`; `observe` reports `Stale` (not the file's state) when the counter hasn't advanced between two reads or the timestamp is older than 5s. File mtime alone is not the freshness signal — the counter is.

### A3. Lock-file takeover races — BREAKS "exactly one parent" under crash loops

PID liveness + mtime freshness has a classic hole: PID reuse (dead PID number recycled by an unrelated process) makes a live lock look dead, and two crash-looping parents can ping-pong the lock, each killing the other's child via handle custody on takeover... actually worse: the old parent is dead, its child died with it; the new parent starts a new child; if the old parent's systemd unit restarts it (Restart=always on both units?), two parents fight.

**Answer:** real hole. Amendment: the lock file holds `(pid, parent_epoch_uuid, started_at_ms)`. Takeover requires the lock holder PID to be dead AND a grace period (2× freshness bound). Additionally: only ONE systemd unit per role may have `Restart=always` — the parent unit. The role child is never itself a systemd service on a parented host (that double-ownership is exactly today's drift). Add a preflight check: parent refuses to start if a `cynic-<role>.service` unit is active ( Gang task: the unit files are the enforcement). The stale-socket problem (option 2) would have been worse — this stays with the file, hardened.

### A4. `restart_count` resets on parent restart — BREAKS the restart drill across parent crashes

The drill (6 kills → GiveUp) assumes one parent lifetime. A parent crash at kill 4 resets the counter; a flapping child + flapping parent never reaches GiveUp and restarts forever. The "records persist" hand-wave doesn't bound anything.

**Answer:** true gap. Amendment: the parent persists `(restart_count, window_start)` to the state dir (`/var/lib/cynic/supervise-<role>.restarts.json`, atomic write) on every restart decision. New parent loads it; entries older than the contract window expire. GiveUp then holds across parent restarts. This file is state, not observation — separate path from the status file, fsync'd before the child spawn. Drill extension: kill -9 the parent mid-sequence and prove the count resumes, not resets.

### A5. The spec keeps `daemon <role>` standalone AND adds the parent — BREAKS single-binary convergence

Two ways to run every role is how the fragmentation started (CLI vs node vs runtime vs MCP). A "temporary" fallback with no removal date is permanent.

**Answer:** fair. Amendment: each rollout step names its removal condition up front (step 1: parent crash rate below child crash rate over 30 days 24/7, then standalone becomes `--standalone-do-not-use` hidden flag for one release, then burns). The convergence map gets a per-step removal row. No removal condition = no step `go`.

### A6. Inference CLI `start` undecided — ACCEPTED as deferred, but it blocks S3b's proof

Step 2 says "decided at build time". If CLI keeps direct-spawn, two custodians exist (parent + CLI) and the lock file only guards parent-vs-parent, not CLI-vs-parent. A user running `cynic models start` beside the parent gets two `llama-server` on one port — second one fails to bind, first one unaffected, but the failure is confusing, not explicit.

**Answer:** accepted as known hole with a floor: S3b lands WITH the decision (no "build time" deferral — that sentence is cut). Floor options at S3b: CLI detects the parent lock and refuses direct spawn with a named error pointing at the parent (`supervisor_parent_holds_role:llama-server`), or CLI proxies to the parent. Refuse-first is the minimal honest behavior. Spec amendment: cut the deferral sentence; S3b includes refuse-when-parented.

### A7. No resource limits on the parented child — ACCEPTED residual

The spec says nothing about memory/CPU bounding (systemd `MemoryMax`, cgroup). A runaway child under a faithful parent gets restarted forever at policy cadence, each time eating RAM until OOM kills something else.

**Answer:** accepted residual, out of scope for the custody spec. The parent owns restart cadence (already bounded by GiveUp); resource bounding stays with the outer systemd unit (`MemoryMax=` on the parent unit covers the subtree via cgroup). Record as a unit-file requirement in step 1, not a parent feature.

### A8. `observe` freshness needs TWO reads for the counter check — BREAKS single-shot CLI semantics

A7's amendment (counter advance between two reads) forces `observe` to read twice with a delay, or to keep local state between invocations. A one-shot CLI printing one file's content cannot detect a frozen counter.

**Answer:** true for frozen-counter detection. Single-shot `observe` reports: file state + `observed_at` age. Frozen-but-fresh-timestamp wedges (parent writing the same counter with a live timestamp loop... actually a wedged parent stops writing, so mtime age catches it; the counter catches replay/corruption cases). Resolution: single-shot uses timestamp age (>5s → Stale); the counter exists for the future watch mode (`observe --watch`) and for post-mortem forensics. No two-read hack in the one-shot path. Amendment recorded.

### A9. Kernel under parent risks boot deadlock — ACCEPTED with ordering guard, needs a drill

Step 3 says `After=` prerequisites, but a parent that fails (bad binary + `Restart=always` + fast crash loop) churns during boot and can delay `multi-user.target` if ordered before it.

**Answer:** accepted risk with guards: kernel parent unit is `After=network-online.target`, never `Before=multi-user.target`, with `StartLimitBurst`/`StartLimitIntervalSec` so a crash-looping parent stops trying instead of churning boot. Plus a dedicated drill on a non-production boot before the 24/7 host takes it. This is why kernel is step 3, not step 1.

### A10. The whole spec assumes Linux paths (`/run`, `/var/lib`) — ACCEPTED, gated by contract

`/run/cynic`, `/var/lib/cynic`, systemd units, PID liveness — all Linux. The contract demands OS-agnostic architecture with per-host implementation.

**Answer:** accepted by design: spec section 2 already scopes implementation to Linux first. Amendment: all Linux-bound paths and mechanisms move behind a `host_scope` module boundary in the implementation slice (`cfg(target_os = "linux")`), with the custody/state-machine core pure and tested without it. The spec's process tree and file paths are the Linux instantiation, labelled as such.

---

## 10. Spec amendments adopted from review

1. Standalone fallback keeps a named removal condition per step; no condition = no `go`. (A1, A5)
2. Freshness = heartbeat counter + timestamp age; single-shot uses age, counter serves watch/forensics. (A2, A8)
3. Lock holds `(pid, epoch_uuid, started_at)`; takeover needs dead PID + grace; child never double-owned by systemd on a parented host; preflight refuses when a role unit is active. (A3)
4. Restart count persists across parent restarts (`/var/lib/cynic/`, fsync before spawn); drill includes parent kill -9 mid-sequence. (A4)
5. S3b includes refuse-when-parented (cut the deferral). (A6)
6. Linux paths behind a `host_scope` boundary; core pure. (A10)
7. Kernel parent unit ordering + start-limit guards; non-production boot drill first. (A9)
8. `MemoryMax` on the parent unit file covers the subtree; not a parent feature. (A7)

With these amendments the spec is implementable step by step. Rollout step 1 (`node` under parent + status file + observe wiring) is the first `go` candidate. G1 closes when step 2 lands; G6 closes when step 4 lands.
