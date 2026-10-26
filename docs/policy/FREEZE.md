# CYNIC Development Freeze — v1

**Tagged:** `freeze/v1` → `e8eb4d74` (2026-10-26)
**Status:** ACTIVE
**Scope:** All development on CYNIC main branch and worktrees.

## Why

The system entered a self-reinforcing feedback loop:

```
more agents in parallel → more churn (refactor, supersede, docs)
  → main breaks + tests don't compile → restore commits
  → more agents to repair → more churn (cycle)
```

Observed over 30 days: 2237 commits, 22% features, 78% churn.
9007 .rs files touched. 4 god objects = 61% of code.
9 active branches, 20 author identities for 1 human.

The freeze stops the loop. The measurement layer (audit churn) prevents recurrence.

## Rules during freeze

### Allowed
- Bug fixes that block compilation or break main (with proof)
- Extraction of one god object at a time (with builder proof)
- Addition of measurement instruments (audit churn)

### Forbidden
- New crates (without proof of need)
- Refactor of surface (rename, reorganize, purge cosmetic)
- Supersede (write then drop)
- New `*_surface` modules
- Docs non-code commits on main
- Parallel agent work on the same scope
- Convergence more than once per day

## The 12 rules (by layer)

### Layer 0 — Substrate
- **S1** Main is always buildable. No commit breaks main.
- **S2** Tests compile before merge.
- **S3** Build SSOT = builder. No local builds.

### Layer 1 — Structure
- **St1** No new crate without proof of need.
- **St2** The 4 god objects are frozen. Extraction: one at a time, with proof.
- **St3** No surface refactor (rename, reorganize, cosmetic purge).

### Layer 2 — Process
- **P1** One agent per scope. No parallel churn.
- **P2** No supersede. Unfinished features don't merge.
- **P3** No "restore main". Don't break it.
- **P4** Convergence once per day max.

### Layer 3 — Content
- **C1** Code repo contains code + technical docs only. No manuscripts.
- **C2** No non-code docs commits on main during freeze.

## Measurement layer

The system must observe itself. Metrics and thresholds live in `infra/churn-thresholds.json`.
The audit `cynic audit churn` computes them and reports against thresholds.

When a threshold is crossed, the audit proposes a rule evolution. It does not mutate — it proposes.

## Post-freeze work (ordered)

1. Scind cynic-protocol (pure types vs predicates)
2. Unify naming convention (audit-enforced)
3. Reduce SSH (native APIs for build/convergence/deploy)
4. Move docs out of code repo
5. Reduce god objects (one extraction at a time, with proof)

## Audit churn

Run: `python3 scripts/audit_churn.py`
Output: metrics + status (OK/WARNING/ALARM) per dimension.

**TODO (post-freeze):** Port to Rust. Python is pragmatic for now, not durable.

## How to lift the freeze

The freeze lifts when:
1. `cynic audit churn` reports OK on all 8 dimensions
2. The measurement layer is operational (audit runs automatically)
3. A PR is opened with the lift proposal and merged via sovereign PR

Do not lift by deleting this file. Lift by evidence.