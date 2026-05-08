# Git Hygiene Sense — Design Spec

**Date:** 2026-05-08
**Status:** Draft
**Approach:** A — Hook extension (session-init.sh + bootstrap script)

## Problem

The organism probes git state at session boundaries (session-init.sh, session-stop.sh) but never posts it as a structured observation. Git metadata — branch count, stash accumulation, PR lifecycle — is ephemeral: visible to the current cortex, invisible to the crystal pipeline. The organism cannot learn patterns about its own git hygiene over time.

Observed today: 29 local branches (25 merged), 5 stashes (some >7 days old), 0 open PRs. This debt accumulated silently because no sense was feeding the data to CCM.

## Design Principles

- **Data-centric.** No hardcoded thresholds. Emit raw metrics every session. Let CCM crystallize what "normal" looks like for this organism. Thresholds emerge from measured baselines, not from assumptions.
- **Bootstrap from history.** 30 days of git history + 165 session logs already exist. A one-time script backfills the time-series so the organism has a baseline from birth.
- **Narrow scope.** Git hygiene only: branches, stashes, PRs. No build state, no session cost, no judgment quality. Those are future organic extensions.

## Architecture

### Component 1: Session-init.sh emission (ongoing sense)

**Where:** After line 132 in `session-init.sh` (after writing session-proof.json, before violation warnings).

**What:** POST a `git_hygiene_sense` observation to `/observe` with raw metrics.

**Schema:**

```json
{
  "tool": "git_hygiene_sense",
  "target": "session_start",
  "domain": "git-hygiene",
  "agent_id": "<agent_id>",
  "context": "<structured metrics — see below>",
  "tags": ["git-hygiene", "sense"],
  "consumer": "ccm",
  "action": "crystallize git hygiene patterns"
}
```

**Context field** (key=value pairs, consistent with existing session-stop.sh convention):

```
branches_local=29 branches_merged=25 branches_unpushed=3 stashes_total=5 stash_max_age_days=12 open_prs=0 stale_prs=0 pruned_this_session=2
```

All values are integers. All are derived from data session-init.sh already collects — no new git commands needed.

**Derivation of each metric:**

| Metric | Source (already in session-init.sh) | How |
|--------|-------------------------------------|-----|
| `branches_local` | `$OPEN_BRANCHES` (line 89) | `jq 'length'` |
| `branches_merged` | new: count branches where `git branch --merged main` | Single git command, ~0ms |
| `branches_unpushed` | new: branches not in `origin/` | `git branch -vv \| grep -v '\[origin/'` |
| `stashes_total` | `$STASHES` (line 98) | `jq 'length'` |
| `stash_max_age_days` | new: parse `git stash list --date=unix` | Oldest stash timestamp vs now |
| `open_prs` | `$OPEN_PRS` (line 94) | `jq 'length'` |
| `stale_prs` | new: PRs with no update >3 days | `jq` filter on `updatedAt` field from `gh pr list` |
| `pruned_this_session` | `$PRUNED_COUNT` (line 70) | Already computed |

**New git commands:** 3 lightweight commands (branches_merged, branches_unpushed, stash_max_age). Total added latency: <100ms.

### Component 2: Bootstrap script (one-time backfill)

**File:** `scripts/bootstrap-git-hygiene.sh` (Tier 1 EXPERIMENTAL — run once, delete after)

**What:** Reconstruct historical git hygiene metrics from git history and POST them as backdated observations.

**Method:**

1. Walk `git for-each-ref --format='%(creatordate:unix) %(refname:short)' refs/heads/` to get branch creation dates
2. Walk `git log --format='%H %ai' --all` to get commit velocity per day
3. For each distinct day in the range, compute:
   - How many branches existed on that date (created before, not yet deleted)
   - Commits that day
   - PRs merged that day (from `gh pr list --state merged --json mergedAt`)
4. POST each day as one `git_hygiene_sense` observation with `created_at` backdated

**Limitations (epistemic: inferred):**
- Stash history is not recoverable from git (stashes are local, no timestamp log). Bootstrap will have `stashes_total=unknown` for historical data points.
- Branch deletion dates are approximate (inferred from merge commit dates, not actual `git branch -d` timestamps).
- PRs before GitHub repo creation won't appear.

**K15 consumer:** CCM crystallization pipeline. The bootstrap observations enter the same pipeline as ongoing emissions — CCM doesn't distinguish historical from live.

### Component 3: Session-init.sh warning (crystal-driven, Phase 2)

**Not built now.** Once CCM has enough data points (~20 sessions post-bootstrap), session-init.sh can read recent `domain=git-hygiene` observations and compare current state to the organism's rolling baseline. If current deviates significantly from baseline, inject a warning.

This is the organic growth path: sense first (emit raw), learn (CCM crystallizes), then act (warn on deviation). No hardcoded thresholds ever.

## Data Flow

```
session-init.sh
  ├── [existing] probe git state (branches, stashes, PRs)
  ├── [existing] write session-proof.json
  ├── [NEW] POST /observe domain=git-hygiene ──→ SurrealDB ──→ CCM crystallization
  └── [existing] inject warnings into conversation

bootstrap-git-hygiene.sh (one-time)
  ├── reconstruct daily metrics from git history
  └── POST /observe domain=git-hygiene (backdated) ──→ same pipeline
```

## What This Does NOT Do

- No thresholds. No "warning if branches > 5."
- No new kernel code. Uses existing `/observe` endpoint as-is.
- No new domain logic. `domain=git-hygiene` is just a string — CCM processes all domains the same way.
- No between-session sensing. Git state is captured at session boundaries only. If the crystal pipeline reveals this gap matters, a cron or kernel sense is the next organic step.
- No build state, no session cost, no judgment quality. Those are separate senses for separate future wires.

## Files Changed

| File | Change |
|------|--------|
| `.claude/hooks/session-init.sh` | Add ~25 lines after line 132: compute 3 new metrics + POST observation |
| `scripts/bootstrap-git-hygiene.sh` | New file (~80 lines): one-time backfill from git history |

## Falsification

**The sense is useful if:** after 20+ sessions, CCM produces at least one crystal in `domain=git-hygiene` that correlates git hygiene metrics with session outcomes (compliance score, commits produced, branch lifecycle patterns).

**The sense is noise if:** after 20+ sessions, all git-hygiene observations remain uncorrelated noise — no crystals form, no patterns emerge. In that case: delete the emission, keep the bootstrap data as historical record, and investigate whether the signal is too coarse (need finer granularity) or too noisy (need different metrics).

## Implementation Order

1. Wire the emission in session-init.sh (immediate value: data starts flowing)
2. Write and run the bootstrap script (backfill: organism gets a baseline)
3. Verify observations land in SurrealDB (`curl /observations?domain=git-hygiene`)
4. Wait. Let CCM do its work. Measure after 20 sessions.
