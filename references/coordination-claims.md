# Coordination Claims — Baseline/Cynic Concurrency

This reference captures the multi-agent coordination claims pattern. Use when
the user asks about agent concurrency, worktree conflicts, or "two modes" of
operation.

## The problem

Multiple agents (Hermes, Codex, CLI) share one CYNIC worktree. The
`build-admission-check` blocks builds when the checkout is mutable — which is
exactly what happens when multiple agents work in parallel. Dirty checkouts
cannot build, test, or get admitted.

## Two modes

| Mode | Who | Enforcement | Preemption |
|------|-----|-------------|------------|
| `baseline` | External agents (Hermes, Codex, CLI) | Best-effort, cooperative. Claims are advisory. | Can be preempted by cynic. |
| `cynic` | Native scheduler workloads | Durably enforced by scheduler. | Can preempt baseline. |

## Claim lifecycle

1. **Claim** — `cynic agent claim <scope>` before mutation.
   - Scope is hierarchical: `fleet` → `node` → `crate` → `module` → `file`.
   - Each claim carries a budget (`actions`, `wall_seconds`) and TTL.
   - Two claims with overlapping scopes conflict. First-wins for same mode;
     cynic preempts baseline.

2. **Heartbeat** — renew TTL while working.
   - Stale heartbeat (no renewal within TTL) → claim expires → others may
     preempt.

3. **Complete** — `cynic agent release <claim_id>`.
   - Sets status `done`, enqueues merge.
   - Merge bot resolves conflicts when multiple claims targeted the same scope.

## Conflict resolution

| Situation | Resolution |
|---|---|
| Same scope, same mode | First claim wins. Second waits or picks another scope. |
| Cynic vs baseline | Cynic preempts. Baseline gets grace period to release cleanly. |
| Batch claim partial failure | Atomic: either all claimed or none. |
| Merge conflict | Rebase against current main; re-claim if scope still needed. |

## Scaling via hierarchy

A `Fleet` claim decomposes into `Node` claims. A `Node` claim decomposes
into `Crate` claims. Parent budget must be >= sum of children budgets
(monotonic). Each child is an independent claim — different agents can work
on different crates simultaneously under the same parent.

## Preemption grace period

When cynic preempts baseline:
1. Cynic writes `preempted_by: <cynic_claim_id>` on baseline claim
2. Baseline agent receives signal (via `cynic agent status`)
3. Baseline has `ttl_seconds` to finish current atomic unit and release
4. If baseline does not release → forced release after grace period
5. Baseline's work is preserved in `baseline_ref` for future reconciliation

## Data model (`.agent-claims.json` at repo root)

```jsonc
{
  "schema": "cynic.agent_claims.v2",
  "revision": 42,
  "claims": [
    {
      "claim_id": "claim_001",
      "agent_id": "hermes:main",
      "mode": "baseline",
      "scope": { "kind": "file", "path": "crates/cynic-kernel/src/lib.rs" },
      "budget": { "actions": 50, "wall_seconds": 3600 },
      "consumed": { "actions": 0, "wall_seconds": 0 },
      "temporal": {
        "started_at_unix_s": 1724000000,
        "ttl_seconds": 1800,
        "heartbeat_at_unix_s": 1724001800,
        "expires_at_unix_s": 1724001800
      },
      "priority": 10,
      "parent_claim_id": null,
      "status": "active"
    }
  ],
  "merge_queue": []
}
```

Pure predicates (`cynic_protocol::coordination_lease::predicates`) handle
scope overlap, expiration, heartbeat staleness, and preemption rules.

## Pitfall

Do NOT create scripts for claim management. CYNIC already has MCP surfaces
(`cynic_coord_claim`, `cynic_coord_release`, `cynic_coord_status`) and CLI
surfaces (`cynic agent claim`, `cynic agent release`, `cynic agent status`).
Scripts reintroduce the adapter pattern we are deleting. Always extend the
existing surfaces.

## Implementation files

- Types: `crates/cynic-protocol/src/coordination_lease.rs`
- CLI: `cynic agent claim/release/status`
- MCP: `cynic_coord_claim/release/status`
- Store: `.agent-claims.json` at repo root
