# Branch-State Audit — Native cynic branch/worktree state (Phase 0, prose only)

Source discipline: painfully-explicit-specs (@TheAhmadOsman on X).
Status: PROPOSAL — human validation required before Rust.
Recipe: cynic-dev-audit-surface-recipe (4 wiring points, derived status,
count-sum invariant, ratchet).

## Why (observed, 2026-09-08)

- 19 remote branches, 0 mergeable as-is: every candidate is DIVERGENTE
  (behind 57–2146) except local main (+20/−0, needs PR+proof) and
  h1-intent-clean (+2, the only healthy branch).
- The skill cynic-branch-lifecycle carries a STATIC classification table that
  is already stale (listed branches do not exist) — a second editorial ledger,
  the exact defect this audit eliminates.
- No cynic surface owns branch/worktree state: no typed BranchState, no
  worktree registry, no JSON read-back. Agents coordinate claims on files
  while branches rot underneath.
- First dataset (this doc is the seed, not the ledger): the 2026-09-08 audit
  table (19 branches, ahead/behind/last-activity). The audit re-derives it
  on every run; this doc never stores statuses.

## What (one read-only surface)

`cynic dev branch-audit --json` → schema cynic.dev.branch_audit.v1:

  branches[]: { name, ahead_of_main, behind_main, last_activity_date,
                derived_status: active | divergente | morte,
                worktree_path | null }
  worktrees[]: { path, branch, head_sha }
  counts: { total, active, divergente, morte } + sum invariant test
  status: ok | blocked (blocked while any DIVERGENTE branch holds
    unmerged unique commits, or while main is ahead of origin/main —
    unpushed main is itself a blocked state per Protocol 0)

Thresholds (evolutive policy, not constitutional): ACTIVE = behind < 50 and
activity < 30 days; DIVERGENTE = behind >= 50; MORTE = no activity > 90 days
or 0 unique commits. Table carries expectations + git tokens only.

## Where (4 wiring points, per recipe)

1. cynic-cli main.rs: `mod branch_audit;`
2. dev_surface.rs: DevCommand::BranchAudit (+ JSON-only guard, persist,
   emit_json).
3. lib.rs: `#[path] pub mod branch_audit;` (else tests never run).
4. Registries same change: surface-registry fragment + agent-exposure
   fallback_commands.
Module excludes ITSELF from its scan. Production zones only (split #[cfg(test)]).

## How

1. Phase 0 (this doc): human validates.
2. Phase 1: module + count-sum invariant test + ratchet (DIVERGENTE visible
   until resolved; invert when clean) + scratch validation with
   CYNIC_SOURCE_ROOT against the real tree.
3. Later: claim/store integration (branch state feeds claim admission:
   refuse claims on MORTE branches), PR-open automation — separate slices.

## Explicit unknowns (owned)

- U1: git access mechanism — owner Phase 1. Audit modules scan files via
  rust_source; branch state needs git argv. Proposal: read-only git via the
  injected ArgvRunner port (no hardcoded `git` spawn in domain code), but the
  audit-surface precedent is file-scan. Due before types.
- U2: worktree enumeration without `git worktree list` parsing fragility —
  owner Phase 1. Proposal: parse porcelain; record fallback blocked_unobserved.
- U3: github/* mirror refs (second remote) in or out of scope — owner human.
  Proposal: origin only; github relay listed as residual unknown until
  sovereignty-relay story settles.

## Gates

- `cynic dev branch-audit --json` runs on the 2026-09-08 tree and reproduces
  this doc's counts (19 branches, 1 healthy) or explains the delta.
- make check green; no new blocked area.
- The skill's static table is retired or regenerated from the audit (no two
  ledgers).

## Rollback

Prose: delete this file. Phase 1: remove module + 4 wiring points, zero
consumers.
