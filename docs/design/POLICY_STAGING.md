# CYNIC Policy Staging — Desired State (proposal)

Registry edits merge to main but nodes keep stale local copies: there is
no registry version, no propagation path, and no read-back. Observed
2026-10-22: compute-01 capabilities merged to main while both nodes kept
serving older copies installed by hand. Policy without propagation is a
wish; propagation without read-back is a rumor.

This document stages policy updates the way BUILD_LIFECYCLE stages
builds: every stage names its owner and its read-back. A stage without
read-back is not done.

## Policy in scope

`infra/registry.json` (nodes, pools, capabilities, required tools),
`infra/runtime-authority.json`, `contracts/` pruning and retention
values. Code ships through the build lifecycle; policy ships through
this one.

## Stages

1. Propose: branch + PR against main, diff limited to policy files.
   Owner: proposing agent. Read-back: PR number, exact file SHAs.
2. Validate: `check` on the merge commit plus the role-parity suite
   (every build_runner keeps the role bar: capabilities, toolchain
   triple, enrollment). Owner: builder fleet.
   Read-back: proof bundle, unknowns empty.
3. Admit: merge to main, fast-forward only, human pushes.
   Owner: operator. Read-back: merge commit on Forgejo SSOT.
4. Propagate: one node at a time in canary order — builder, then
   agent-hosts, then cortex. The node installs the admitted copy,
   validates it parses, and reports the applied revision.
   Owner: Hermes-Agent over SSH. Read-back: per-node applied revision.
5. Verify: the fleet reports one revision. Skew pages a human; the
   skewed node keeps serving but is excluded from placement until it
   converges. Owner: submitter health path.
   Read-back: fleet revision table, zero skew.
6. Rollback: revert-forward on main, then re-propagate. Never edit a
   node copy by hand outside propagation — a hand edit is unreported
   drift and fails verification. Owner: operator, human only.

## Minimal mechanism

- The registry carries `revision`: the merge commit SHA that admitted
  it. Nodes expose `applied_revision` in their read-back surfaces.
- Placement prefers revision-current nodes; a stale node stays
  eligible but is reported, never silently served.
- No autonomous push to nodes: propagation runs node by node with
  explicit per-node confirmation, the same `--yes` discipline as
  every other mutation.

## Validated decisions

- D1 canary order reuses the build lifecycle: builder, agent-host,
  cortex.
- D2 hand edits on node copies are drift, not fixes. The fix always
  lands on main first, then propagates.
- D3 rollback is human only, revert-forward.

## Explicit unknowns

- Transport for propagation (scheduler workload kind vs direct
  Hermes-Agent SSH): undesigned.
- Revision comparison point (submitter-side vs scheduler-side):
  unmeasured.
- Propagation queue behavior when a node is offline at its turn:
  undesigned — current default is queue and replay.
