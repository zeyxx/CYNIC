# Sovereign VCS Study — Scion vs CYNIC

Date: 2026-10-24
Status: study (read-only, no code)
Source: Anthropic, "Redesigning source control" (Scion), 2026-10-13

## 1. Why this study

CYNIC already exhibits the agent-scale symptoms Scion was built to solve:
many divergent branches, union-resolution corruption, shared worktree races,
merge-queue stalls. Before deciding whether to build or buy, map Scion's
principles onto CYNIC's existing surfaces and name the real gaps.

## 2. Scion principles (from the article)

1. Changeset model, not commit/refs: immutable, content-addressed; no refs move
   during ingestion → high throughput.
2. Derived data (blame, history) computed lazily/recursively, not at commit
   → avoid materializing what is not read.
3. Server is authority with server-side RPCs; client no longer negotiates
   packfiles → tiered caches, 99% hit, 200x smaller footprint.
4. No global merge-queue lock → post-merge / pre-deploy testing; conflicts
   resolved by the agent.
5. Sparse / virtualized checkouts native (client asks only for wanted trees).
6. Per-directory permissions, not just per-repo.
7. Explicit caveat: Scion is NOT for general use; value is the shared thinking.

## 3. Mapping Scion → CYNIC (existing surfaces)

| Scion principle                | CYNIC surface (existing)                          | Coverage |
|-------------------------------|---------------------------------------------------|----------|
| Server-side authority          | Forgejo sovereign + `cynic source *` (read-only)   | Covered  |
| No global lock, agent resolves | claims TTL, per-workload fences, post-merge proof  | Covered  |
| Sparse checkouts               | per-SHA worktrees, `cynic source worktree`        | Covered  |
| Per-directory permissions      | capabilities registry, scope claims               | Covered  |
| Content-addressed derived data | CoherenceVector / tension (cynic-eval)            | Partial  |
| Lazy derived data              | not implemented (blame/history eagerly computed)   | Gap      |
| Server-side RPCs               | `cynic source *` is read-only observer only        | Gap      |
| Changeset model (no refs)      | git commits/refs unchanged                        | Gap      |

## 4. Real gaps (3, bounded)

G1. Lazy derived data. CYNIC computes blame/history/path data eagerly.
    Scion computes them recursively on demand. CYNIC has the hook point
    (CoherenceVector) but no lazy resolver.

G2. Server-side RPCs. `cynic source *` observes; it does not serve. A
    content-addressed read RPC (tree/blob fetch by hash) would let checkouts
    skip packfile negotiation.

G3. Sparse checkout is manual. Worktrees are full clones; there is no
    "fetch only wanted trees" path.

## 5. What is already covered (do not rebuild)

- Sovereign store = Forgejo (not GitHub projection).
- Read-only observer with remote-role discovery (cynic source status).
- Worktree lifecycle, claims, fences, post-merge proof.
- CoherenceVector as the tension/derived-data seed.

## 6. Recommendation (Option 1 now, Option 2 as documented tranche)

Short term (now): keep Git + Forgejo, harden process. Finish current
convergence. Archive dead branches. Do not start a VCS rewrite.

Medium term (documented tranche, not started): a Rust service extending
cynic-vcs that exposes content-addressed read RPCs and lazy derived data,
with Git as storage backend. Estimated 4-8 weeks. Starts only after
convergence lands.

Long term: full sovereign VCS only if agent volume forces it. The article
itself says this is only justified for very large monorepos.

## 7. Explicit non-goals

- No VCS rewrite now.
- No abandonment of Git/Forgejo.
- No "Scion clone" — Anthropic has 7 engineers and bespoke infra; CYNIC does
  not. Extract principles, not code.
- No code in this tranche. This document is the deliverable.

## 8. Next bounded step

When convergence is green: open a design tranche for G2 (server-side read RPC)
first, as it has the highest leverage and the lowest risk. G1 and G3 follow.
