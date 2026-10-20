# Branch convergence map — 2026-10-20

A read-only census of every `agent/*` branch on the Forgejo remote, measured
before anything was merged. 175 branches exist. 142 of them carry nothing that
`main` does not already have. 33 carry work. Of those 33, **19 are fully
contained in a single aggregate branch**, and the aggregate branch itself is
the only real merge candidate in the `5c47b650` family.

This document exists because the previous convergence attempts worked from the
PR list. The PR list is the subset somebody chose to propose. The census is
the remote ref list, and the difference is where the work was hiding.

## How the census was made

Read-only, in this order:

```bash
git ls-remote origin 'refs/heads/agent/*'                    # 175 refs
git rev-list --count origin/main..origin/<branch>            # commits ahead
git rev-list --count origin/<branch>..origin/main            # commits behind
git merge-base origin/main origin/<branch>                   # shared base
git diff --name-only <base>..origin/<branch>                 # changed set
git rev-list --count origin/<branch> ^origin/<aggregate>     # containment
git cherry origin/main origin/<branch>                       # patch-equality
```

Two rules, both learned the hard way:

- **Compare changed sets against the shared merge-base, not against `main`.**
  Eleven branches share the base `5c47b650`, which is 275 commits behind
  `main`. An intersection computed against `main` counts `main`'s own drift as
  common and inflates the overlap.
- **Containment before proposal.** `git rev-list --count <B> ^<A>` equal to
  zero means B is fully contained in A. Merging B into A is then an empty
  gesture; the correct action is to archive B, not to merge it.

## The shape of the debt

| Measure | Value |
|---|---|
| Remote `agent/*` branches | 175 |
| Fully merged into `main` (nothing to do) | 142 |
| Carrying work not in `main` | 33 |
| Fully contained in `agent/hermes/aegis-consortium` | 19 |
| Independent merge candidates | 12 (+ the aggregate itself) |

### The aggregate

`agent/hermes/aegis-consortium` is 259 commits ahead of `main`, 126 behind,
185 changed files, and 203 commits that are not in `main` by patch equality.
It is a convergence branch, not a feature branch: its own history contains
merge commits from `supervision-convergence`,
`single-binary-topology-writer`, `shannon-wiring-main-fix`, `probe-compute`
and `toolchain-kinds`.

`agent/robinho100/epistemic-engine` is the same work under a second name —
same 259 commits, 185 shared files, and exactly two commits each way that the
other does not have. It is a duplicate, not a second candidate.

### What is contained (19 branches — archive, do not merge)

Each of these has **zero** commits that `aegis-consortium` does not already
have. Merging them is a no-op.

`base-enrollment`, `build-placement`, `compute-01-build`,
`convergence-debt-tests2`, `debt-tests2`, `gate-eligibility-fix`,
`gate-receipts`, `inference-vertical-chat-logprobs`, `pool-capacity-gate`,
`rag-dense-margin`, `scheduler-truths-t5`, `shannon-information-primitive`,
`shannon-integration`, `shannon-wiring-main`, `shannon-wiring-main-fix`,
`single-binary-topology-writer`, `supervision-convergence`,
`sub/compute-capacity`, `sub/probe-compute`.

`compute-01-build-v5` is a special case: one commit ahead by SHA, but
`git cherry` reports it as already in `main` (0 to merge, 1 already there).
It is a cherry-pick that was re-created as a branch. It carries nothing.

## Convergence order

The aggregate merges FIRST and alone. Merging its inputs separately first
would replay the same content twice.

| # | Branch | ahead | behind | files | Role |
|---|---|---|---|---|---|
| 1 | `agent/hermes/aegis-consortium` | 259 | 126 | 185 | the aggregate: supervision, single-binary topology, shannon, rag, scheduler truths, debt tests |
| 2 | `agent/hermes/fleet-chat-inference-2` | 54 | 416 | 124 | node surface split into noun families and verbs |
| 3 | `agent/hermes/cd-couche-b-smoke` | 37 | 60 | 30 | deploy location SSOT, release crate |
| 4 | `agent/hermes/pool-toolchain-kinds` | 9 | 127 | 5 | runner state root for the target dir |
| 5 | `agent/hermes/agent-messaging-spec` | 4 | 129 | 5 | idea domain, session-context spelling |

The four `shannon-*` branches plus `inference-vertical-chat-logprobs` touch
the same 6 files with identical content and are mutually contained — they are
one work item under five names. They are part of the aggregate (contained)
and need no separate PR.

`fleet-chat-inference-2` overlaps the aggregate on `cynic-cli` and
`cynic-node`; it is scheduled after it for that reason, and the merge must be
proven green, not assumed.

## The coordination protocol this reveals

Three failure modes produced the 175 branches, and each has a rule that
prevents it:

**1. Duplicate work under two names.** `aegis-consortium` and
`epistemic-engine` are 185 files of the same 259 commits. Two agents, one
work item, neither knew about the other. *Rule: a branch that touches a file
another branch touched in the same window is a candidate for containment
check, not for a parallel PR.* The check is one command:
`git rev-list --count <A> ^<B>`.

**2. Work parked in an aggregate that never merged.** Nineteen branches were
merged into `aegis-consortium` and the aggregate itself was never proposed.
The work existed, was tested, and was invisible — 203 commits of it, never in
`main`. *Rule: an aggregate branch is a merge candidate on its own schedule.
Merging inputs into it is not "getting the work in".*

**3. Recreated branches.** `compute-01-build-v5` is a branch whose content is
already in `main` — re-created rather than recognised as merged. *Rule: judge
by patch equality (`git cherry`), not by SHA distance. A branch can be "1
ahead" and carry nothing.*

**What a chaos session leaves behind, measurably:** 142 branches of pure
noise (81 % of the remote), one aggregate that nobody proposed, and 12 real
candidates. The noise is cheap to delete and expensive to leave — it makes
every future census slower and hides the two or three branches that matter.

## What this document does not decide

- The order inside the aggregate is not addressed: it is one branch, and its
  internal history is already merged in the order its author chose.
- `epistemic-engine`'s two unique commits (a revert of an audit fix, plus its
  inverse) need a decision of their own — they are a revert cycle, not
  content.
- The 142 fully-merged branches are candidates for deletion, not for merges.
  Deleting a remote ref is a mutation on the operator's estate: it is listed
  here, not performed.

## Verification

Every number in this document comes from the commands quoted above, run
read-only against `origin` on 2026-10-20. No branch was merged, rebased, or
deleted to produce it.
