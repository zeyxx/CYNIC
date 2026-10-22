# Organic construction priority — from observed data, 2026-10-22

Source: live fleet state, not opinion. Every claim below names its
record. Missing measurements are declared as gaps, never filled by guess.

## What the fleet emitted today

- Builder settlements (last window): 16 settled — 13 completed, 3 failed.
  All 3 failures: `build_exit_code:Some(101)`, no stderr, no log ref.
  Median elapsed 109s, range 3–195s.
- compute-01 workloads: 5 — 2 completed, 3 blocked
  (`source_bootstrap_repository_not_git`,
  `workspace_layout_not_declared`,
  `build_execution_spawn: Permission denied`).
- Enrollment: grant TTL 50min → `:9090` dark ~2h, unit `active` throughout.
  No renewal attempt record anywhere; renewal endpoint (`127.0.0.1:3030`)
  unreachable by construction.
- Disk: builder 69% full (workloads 7.7M, evidence 920K — small files,
  big disk: the weight is elsewhere, unmeasured per class);
  compute-01 `/var/lib/cynic` 3.0G (build-targets 483M, bin 231M,
  source 68M, workspaces 29M).
- Builder holds ~150 workload dirs; retention policy unknown.
  compute-01 identity/: 18 files (ceremony debris included).

## Declared measurement gaps

1. Build failure cause: settlement records the exit code but nothing
   that distinguishes a typo from an ICE from a missing dep.
2. Renewal attempts: the loop only `tracing::warn`s to a root-only journal.
   No countable outcome, no last-error surface while the grant is valid.
3. Failure recurrence: no count of same-commit/same-action repeats.
   Cannot tell a flake from a broken main.
4. Per-class disk: no budget, no attribution (which class eats the 69%?).
5. Process lifecycle: agent-spawned processes have no owner, no reaper.
6. Decision trace: no link from a measurement to the action it triggered
   (e.g. enrollment-check → re-issuance lives only in chat).

## Priority order (organic: lower layers first)

### P0 — Enrollment continuity (outage proven)
Sub-problems:
- Renewal endpoint unreachable from build_runner nodes by construction.
- Renewal failure invisible until the grant lapses (surface goes dark).
- Grant TTL chosen by accident (request expiry caps the grant).
Fixes:
- DONE this session: 90d grant, ceremony doc, `enrollment-check` arm.
- NEXT: remote renewal design (authority reachable over transport) or
  push model; until then TTL policy per role + alerting on
  `renewal_endpoint_reachable == false`.

### P1 — Build failure evidence (3/16 blind)
Sub-problems:
- Settlement lacks stderr tail / rustc version / failing crate.
- No recurrence counting per commit+action.
Fixes:
- Extend settlement (or linked evidence ref) with: action, commit,
  failing crate, stderr tail (bounded, e.g. 2KB), rustc version.
- Recurrence counter; repeated same-commit failures page, don't just log.

### P2 — compute-01 execution blockers (second builder unreliable)
Sub-problems: the 3 blocked unknowns above, one by one.
Fixes:
- `source_bootstrap_repository_not_git`: declare or fix the bootstrap path.
- `workspace_layout_not_declared`: declare the layout for build_runner.
- `Permission denied` on spawn: sandbox/userns policy for the runner.
- Then: same `check` gate as the builder before it takes build load.

### P3 — Lifecycle GC (growth without bounds)
Sub-problems: 150 workload dirs, binary sprawl (4 cynic variants),
identity debris, logs without rotation, evidence without TTL.
Fixes:
- One rule per class: TTL + reconciler that purges (leases and claims
  already have TTLs — extend the pattern, don't invent a new one).
- Binary manifest: one live path per artifact, rest archived or removed.
- Process supervision: every agent-spawned process gets an owner or a
  reaper; no more orphan `--check`.

### P4 — Registry/reality convergence (drift by hand)
Sub-problems: single `role` string vs dual reality; manual edits;
no auto-propose.
Fixes:
- DONE this session: compute-01 services[] + grant evidence.
- NEXT: multi-role schema; grant-presenting reconciler that proposes
  capacity-role moves, operator signature for authority roles
  (design recorded in session log 2026-10-22).

### P5 — Decision trace (close the loop)
Sub-problems: measurements don't link to the actions they caused.
Fixes:
- Evidence refs on every mutation (which check, which grant, which prove).
- The execution log becomes a projection of linked records, not chat memory.

## Rule

No new layer until the layer below reports green on its own data.
`enrollment-check`, `service prove`, settlement unknowns are the gates —
not prose, not intent.

## Annex — layer-by-layer reading (code × emergences, 2026-10-22)

The constitution orders L0 → L4, intent down, proof up.
Code gives each layer its owner; today's emergences give each layer
its color. Priority = lowest red layer first.

- L0 pure protocol (cynic-protocol, cynic-node-protocol): GREEN.
  Extended today, 17/17 tests. Priority: keep it pure, no I/O creep.
- L1 host substrate (cynic-host, profiles, sandbox, supervision): RED.
  Renewal endpoint unreachable by profile construction; spawn
  `Permission denied` on compute-01; no process owner/reaper.
  Priority 1 — nothing above is stable while this bleeds.
- L2 domains (cynic-build, node use cases, enrollment): YELLOW-RED.
  Ceremony works, but grant TTL was accidental; 3/16 builder failures
  carry an exit code and no cause. Priority 2 — capture the why.
- L3 orchestrators (scheduler, lifecycle, reconciliation): YELLOW.
  Blocked states accumulate without recurrence counting or auto-propose.
  Priority 3 — reconcile, don't just record.
- L4 projections (CLI, MCP, TUI): GREEN.
  `enrollment-check` and `prove` added today; MCP stays a projection.
  Priority last — surfaces expose, they don't repair.

