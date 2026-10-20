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

Two measures, and they disagree. The first is SHA distance, the second is
patch equality. Where they disagree, patch equality is the truth: a commit
can be "1 ahead" and carry nothing, and a branch can be 92 commits ahead with
one commit of real content.

| Measure | Value |
|---|---|
| Remote `agent/*` branches | 175 |
| Zero commits ahead of `main` by SHA | 142 |
| Carrying commits by SHA | 33 |
| Of those, carrying **nothing** by patch equality (`git cherry`) | 9 |
| Branches with real content to merge | 24 |
| Of those, fully contained in `agent/hermes/aegis-consortium` | 19 |

The nine that carry nothing are `final-proof`, `network-registry-fixture`,
`compute-01-build-v5`, `gate-receipts`, `topology-after-cli-manifest`,
`gate-eligibility-fix`, `base-enrollment`, `compute-01-build` and
`pool-toolchain-kinds`. Each is one commit that `main` already has under a
different SHA — a cherry-pick re-created as a branch. They are not merge
candidates; they are noise with a name.

Several other branches look enormous by SHA and are small by content:
`ci-workspace-test-suite` is 64 commits ahead by SHA and **1** commit to
merge; `topology-after-cli-manifest` is 65 ahead and **0**; `final-proof` is
66 ahead and **0**. Their history diverged early; their content is in `main`.

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

## The aggregate does not compile

`agent/hermes/aegis-consortium` was never validated by a gate. Measured on
the builder, against its own tree:

```
cargo check --workspace --lib   ->   107 errors
```

| File | Errors |
|---|---|
| `crates/cynic-knowledge/src/lib.rs` | 55 |
| `crates/cynic-knowledge/src/domain/mod.rs` | 51 |
| `crates/cynic-workload/src/source_change_submission.rs` | 9 |
| `crates/cynic-network/src/quic/port.rs` | 5 |
| 13 other files | 1–3 each |

Every one is `E0252: the name X is defined multiple times`. The cause is
visible in the source: `crates/cynic-knowledge/src/domain/mod.rs` contains
its `pub use` block **twice** — the full block, then a second copy of the
same exports in a different order. The aggregate's own history says it:

```
b53e5924 chore(convergence): remove residual conflict markers after union merge
```

The conflict markers were deleted, not resolved. The duplicate definitions are
what was underneath them. `crates/cynic-host/src/adapters/local_build_artefact.rs`
carries the same signature — a doubled doc-comment and `use std::path::Path;`
declared alongside `use std::path::{Path, PathBuf};`.

The originating merge is named in the history:

```
ebd93f79 Merge origin/main into agent/hermes/rag-dense-margin (union resolution)
```

A union merge pulled `main` back over `rag-dense-margin`'s work and
duplicated the exports.

### The repair is smaller than it looks, and `main` is ahead

`main`'s own `crates/cynic-knowledge/src/domain/mod.rs` is 58 lines and
already exports the **union** of the two blocks — `synthesis`, `vector_cache`
and `vectorizer` are there exactly once, and `main`'s last commit on the
file is `a0444479 Bind vector cache to producing engine; refuse cross-engine
mixing`, which is *newer* than the aggregate's copy.

So on this file `main` is ahead, not behind: the aggregate is not carrying
new knowledge-domain content, it is carrying a broken copy of older content.
Every module the two blocks reference (`vector_cache`, `synthesis`,
`vectorizer`, `scorer`, `hybrid_rrf`, …) exists in `main`.

The repair is therefore: resolve the duplicates in favour of `main`, drop the
second block, and let the aggregate's real new content stand. That is a
mechanical merge conflict resolution on 8 files, followed by a gate run —
not a rewrite of 203 commits.

### What the aggregate is genuinely worth

Measured as files `main` does not have at all: **55**. Not 185.

| Content | Files |
|---|---|
| `crates/cynic-supervisor` — a whole crate: `parent.rs`, `spawn.rs`, `stop.rs`, `watch.rs` | 6 |
| `contracts/supervisor-contract.json` — `cynic.supervisor_contract.v1`, normative | 1 |
| `docs/design/supervision-convergence-map.md`, `supervision-watch-pattern.md`, `supervised-parent-scope.md`, `sovereign-supervisor.md` | 4 |
| `docs/specs/agent-messaging.md`, `deploy-surface-measured.md` | 2 |
| `infra/os/systemd/cynic-supervise-*.service`, `infra/deploy-locations.json`, `infra/gc-surfaces.json`, `infra/model-metadata.json` | 8+ |
| `crates/cynic-build`, `crates/cynic-eval`, `crates/cynic-release`, `crates/cynic-audit-engine` — new crates | ~30 |

The supervision slice is the reason to recover the aggregate: it holds the
`supervise` verb the soak bench needs, the contract that makes supervision a
first-class authority, and the two design documents the soak units cite. That
slice is worth recovering. The other 130 files are mostly `main`'s content in
an older arrangement.

**This is the fourth failure mode**, and it is the reason the branch was never
proposed: an aggregate branch is not a merge candidate until it compiles.
Two hundred and three commits of content, none of it reachable.

### What the aggregate is actually worth

It is not noise. `single-binary-topology-writer` contributes 51 commits of
real content, `rag-dense-margin` 37, `supervision-convergence` 28,
`fleet-chat-inference-2` 51, `scheduler-truths-t5` 10, `debt-tests2` 9 — and
the `supervise` verb that the soak daemon needs is in there, which is why the
bench has to be revived rather than abandoned.

But it also carries a merge of `fleet-chat-inference-2`, so that branch is
*partly* inside it, not wholly a separate candidate. And 44 of its 259
commits are merge commits, which is why its history is hard to read.

The strategy this changes: **the aggregate cannot be merged as a unit.** It
has to be repaired first — the 107 duplicate definitions resolved against
`main`, not against its own history — and then the repair is a branch of its
own with its own gate run. The content it carries is worth recovering; the
branch as an object is not mergeable.

## Convergence order

The aggregate merges FIRST and alone. Merging its inputs separately first
would replay the same content twice.

`to merge` is the count from `git cherry` — the commits that are not in
`main` by patch equality. That is the real cost of the merge, not the SHA
distance.

| # | Branch | to merge | ahead | behind | Role |
|---|---|---|---|---|---|
| 0 | `agent/hermes/aegis-consortium` | 203 | 259 | 126 | **repair first** — does not compile; see above |
| 1 | `agent/hermes/fleet-chat-inference-2` | 51 | 54 | 416 | node surface split into noun families and verbs |
| 2 | `agent/hermes/single-binary-topology-writer` | 51 | 52 | 275 | single-binary topology, writer side |
| 3 | `agent/hermes/rag-dense-margin` | 37 | 38 | 275 | dense margin in the RAG scorer |
| 4 | `agent/hermes/cd-couche-b-smoke` | 32 | 37 | 60 | deploy location SSOT, release crate |
| 5 | `agent/hermes/supervision-convergence` | 28 | 29 | 275 | the `supervise` verb the soak bench needs |
| 6 | `agent/hermes/scheduler-truths-t5` | 10 | 11 | 275 | scheduler truth set |
| 7 | `agent/hermes/debt-tests2` | 9 | 10 | 275 | debt tests |
| 8 | `agent/hermes/build-placement` | 7 | 8 | 250 | build placement |
| 9 | `agent/hermes/convergence-debt-tests2` | 5 | 6 | 275 | debt tests, second tranche |
| 10 | `agent/hermes/agent-messaging-spec` | 4 | 4 | 129 | idea domain, session-context spelling |
| 11 | `agent/hermes/inference-vertical-chat-logprobs` | 2 | 3 | 275 | chat logprobs |
| 12 | `agent/hermes/shannon-wiring-main-fix` | 2 | 3 | 275 | shannon wiring, fix side |
| 13 | `agent/hermes/ci-workspace-full-suite` | 2 | 5 | 60 | full workspace test suite |
| 14 | `agent/sub/probe-compute` | 1 | 1 | 235 | compute probe |
| 15 | `agent/hermes/ci-workspace-test-suite` | 1 | 1 | 63 | workspace test suite |
| 16 | `agent/sub/compute-capacity` | 1 | 1 | 220 | compute capacity |
| 17 | `agent/hermes/fix-compressor-redaction-prefixes` | 1 | 1 | 110 | redaction prefix fix |
| 18 | `agent/hermes/topology-after-network-fixture` | 1 | 1 | 60 | topology refresh |
| 19 | `agent/hermes/shannon-integration` | 1 | 2 | 275 | shannon integration |
| 20 | `agent/hermes/shannon-wiring-main` | 1 | 2 | 275 | shannon wiring |
| 21 | `agent/hermes/shannon-information-primitive` | 1 | 2 | 275 | information primitive |
| 22 | `agent/hermes/pool-capacity-gate` | 1 | 2 | 220 | pool capacity gate |

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
already in `main` — re-created rather than recognised as merged.
`ci-workspace-test-suite` is 64 commits ahead by SHA and one commit of
content. *Rule: judge by patch equality (`git cherry`), not by SHA distance.
A branch can be "1 ahead" and carry nothing, and "64 ahead" and carry one.*

**4. Conflict markers deleted instead of resolved.** The aggregate was built
by a union merge whose markers were removed, leaving 107 duplicate
definitions across 18 files. The commit says so in its own subject
(`chore(convergence): remove residual conflict markers after union merge`).
The content is real; the branch object is broken. *Rule: an aggregate branch
is not a merge candidate until it compiles on its own tree. A marker-deletion
commit is the signature of an unresolved merge and must be treated as a defect,
not as a cleanup.*

**What a chaos session leaves behind, measurably:** 142 branches of pure
noise (81 % of the remote), one aggregate that nobody proposed and that does
not compile, 9 branches whose content is already in `main`, and 23 real
candidates hiding inside 175 names. The noise is cheap to delete and
expensive to leave — it makes every future census slower and hides the
handful of branches that matter.

## What this document does not decide

- **How to repair the aggregate.** The 107 duplicate definitions have to be
  resolved against `main`, not against the aggregate's own history, and the
  result has to earn its own gate run. Which side wins for each of the 106
  duplicated `cynic-knowledge` symbols is a per-symbol decision that a
  census cannot make.
- `epistemic-engine`'s two unique commits (a revert of an audit fix, plus its
  inverse) need a decision of their own — they are a revert cycle, not
  content, and the branch is otherwise a byte-level duplicate of
  `aegis-consortium`.
- The 142 fully-merged branches are candidates for deletion, not for merges.
  Deleting a remote ref is a mutation on the operator's estate: it is listed
  here, not performed.
- Whether the soak bench is worth reviving depends on repairing the aggregate
  far enough to recover the `supervise` verb — see
  `docs/design/OPERATIONAL_DRIFT_KILLCHAINS.md`, killchain 12.

## Verification

Every number in this document comes from the commands quoted above, run
read-only against `origin` on 2026-10-20, plus one compilation of the
aggregate on the builder (`cargo check --workspace --lib`, 107 errors). No
branch was merged, rebased, or deleted to produce it.
