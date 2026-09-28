# CYNIC — Crate Rules: the pattern, the evidence, and the reasons

- **schema**: `cynic.crate_rules.v1`
- **status**: `reasoning_document` — **non-normative**. This file carries the *why*.
  Authority lives in `contracts/pruning-policy.json` (normative) and in the
  executable audits named in §6. Where this document and an audit disagree,
  **the audit wins**. A stale sentence here is a bug to fix, not a rule.
- **written**: 2026-09-28, from measurements taken that day. Every number below
  was measured on this repository; none is estimated.
- **Why this file exists**: the workspace has ~14 `contracts/*.json`, 29
  `infra/*.json` and a pile of prose, all claiming to describe the rules, none
  of them explaining *why* the rules are shaped this way. An agent reading
  only the rules has to re-derive them, and re-derivation is where the drift
  comes from. This is the reasoning, once, with the numbers attached.

---

## 1. The one-sentence rule

> **A crate is one named responsibility. A concept has exactly one home. Growth
> is expressed by adding crates, never by thickening one. The dependency
> direction is a layered DAG; the growth pattern is a branching tree. Those are
> two different axes and must not be confused.**

Everything below is either a justification for that sentence or a consequence
of it.

## 2. Why growth is a tree, and why the tree is NOT the dependency graph

`contracts/pruning-policy.json` → `split_criteria.crate_justified` has five
conditions and **none of them is "is a binary"**. That is deliberate, and an
earlier draft of this reasoning got it wrong: a binary is a crate with a `main`.
A library is the normal case. 47 crates producing 1 binary is the healthy shape,
not a contradiction.

The branching figure that motivates this document is a Fibonacci tree: a trunk
splits, each branch splits again at a slightly later generation, and the number
of tips at each depth follows 1, 1, 2, 3, 5, 8, 13, 21. Two properties matter:

1. **Self-similarity** — the same shape at every scale. So the rule is written
   once and enforced everywhere: one `tests/architecture_boundary.rs` shape for
   every crate, one state machine for every crate.
2. **Growth adds tips, never thickens the trunk.** A crate that reaches its
   bound does not get bigger; it splits. That is the whole content of the
   ratchet: a crate may never grow past its recorded size.

**The tree cannot be the dependency graph.** A strict tree allows at most one
parent per crate. Measured: 26 of 47 crates have more than one dependency, and
`cynic-protocol` has 26 consumers. Forcing a tree would force the shared
vocabulary to be duplicated down each branch — which is precisely the defect
documented in §4. So:

- **tree = the law of growth** (who splits, into how many, with what shape)
- **layered DAG = the law of dependency** (who may depend on whom)
- **canonical = the law of vocabulary** (who owns a concept)

Measured on this workspace: **0 dependency cycles**, in the Cargo graph and in
the source-level import graph. What the workspace has is not circularity, it is
**concentration** — `cynic-protocol` used by 26 crates, `cynic-cli` importing 19.
A DAG is the right shape; the pain is a missing boundary, not a loop.

## 3. Why line count is not the criterion

The original instinct was a 25 000 LOC ceiling. To test it, a sixth state was
added to the state machine: `SURCHARGE` — a crate that is too large **without**
being multi-responsibility.

**`SURCHARGE` is empty. Zero crates.**

Every crate above the ceiling is also above the responsibility bound. The LOC
axis therefore carries **no independent information**: it is a projection of the
real fault, not the fault. A ratchet on lines would have watched a symptom
while the cause walked past it — and it would have missed the far more common
case of a small crate holding five unrelated things.

What line count *is* good for: blast radius. If a crate is deleted, how much
does it remove. Keep it as a guard, demote it from a design rule.

The reason the belief was so durable: the five biggest crates really do look
like "too big". But the measurement behind the number is different —

```
cynic-host        124 responsibilities   90 905 LOC   →   733 LOC/resp
cynic-protocol    106 responsibilities   33 975 LOC   →   320 LOC/resp
cynic-cli          86 responsibilities   35 448 LOC   →   412 LOC/resp
cynic-node         59 responsibilities   58 992 LOC   →   999 LOC/resp
cynic-inference    13 responsibilities   25 611 LOC   →  1970 LOC/resp
```
Median across the workspace: **6 responsibilities per crate**. Crates with
exactly one responsibility: **2 of 47**. The giants are not too large; they
are 124, 106, 86 and 59 responsibilities with no boundary between them.
`cynic-inference` is the instructive exception: large, but ~2000 LOC per
responsibility — the healthy shape of a large crate.

## 4. Why "one crate, one responsibility" is the rule that is violated

The rule is already ratified (`pruning-policy.json` → `responsibility_rules`:
`clear_owner`, `public_surface_contract`, `one_way_dependency`,
`domain_no_infrastructure_by_default`). It is applied almost nowhere, and the
evidence is duplication, not size:

- **`Claim` has five homes.** `cynic-protocol::agent_coordination` (canonical),
  `cynic-types`, `cynic-agent-bus` (whose `ClaimStatus` is byte-identical to L0's
  enum and *contradicts* it on identity, budget width and consumed shape, while
  missing `fencing_token` and `lease_id`), workspace claims in `cynic-node`, and
  the CLI claims file. Five answers to one question, and none of them is the one
  that runs.
- **`Scope` has three homes** (`AgentScope`, `ClaimScope`, `ScopeKind`),
  **`Budget` has three** (`AgentBudgets`, `Budget`, `Claim.budget`).
- The duplication is always a symptom of the same missing thing: when a shared
  contract did not exist at the moment two layers needed it, each grew its own.

A second, independent defect hides inside the same measurement: **45 022 LOC in
this workspace are never compiled** (361 227 on disk, 316 205 compiled). Dead
mass is not confined to dead crates — it sits *inside live ones*, where no
compiler, no warning and no test can see it. This is why the
`orphan_sources_ratchet` exists and why the audit is reachability-first.

## 5. The state machine

Six states. Every transition has a guard, a measurement, and a direction that
is checkable.

| state | measured as | count (2026-09-28) | example |
|---|---|---|---|
| `SAIN` | compiled · ≥1 dependent or a binary · within the responsibility bound | 20 | `cynic-types` |
| `FRACTURE` | compiled, responsibilities accumulated without a boundary | **16** | `cynic-host` (124) |
| `UNJUSTIFIED` | 0 dependents, not a binary, no recorded disposition | **3** | `cynic-kernel` |
| `PARKED` | a reason is recorded in `pending_disposition` with a verdict file | 8 | `cynic-market` |
| `DEAD` | nothing compiles it | **0** (at crate level; 45 022 LOC at file level) | — |
| `CANDIDATE` | proposed, not yet created | — | `cynic-api` (planned) |

```
CANDIDATE --[5 conditions of crate_justified + named responsibility
             + entry in infra/crate-topology.json]--> SAIN

SAIN      --[responsibilities over the bound, shrink-only recorded]--> FRACTURE
FRACTURE  --[extraction: each child one responsibility, compiled, has a
             dependent]--> SAIN

SAIN      --[last dependent removed, not a binary, no disposition]--> UNJUSTIFIED
UNJUSTIFIED --[reason written into pending_disposition]--> PARKED
PARKED    --[verdict names the owner, references repointed]--> ABSORBED
ABSORBED  --[files deleted, ratchet updated, compiled census falls]--> RETIRED

any       --[another crate declares the same concept]--> CONTESTED
CONTESTED --[the canonical owner wins, the copy is deleted]--> SAIN
```

`CANDIDATE → SAIN` is the only transition that creates anything, and it is
gated by the ratified criteria plus one addition: **the responsibility must be
written down, with its reason, before the crate exists.** A crate nobody can
name is the failure mode this whole document is about.

`UNJUSTIFIED` deserves a note because it is uncomfortable: `cynic-kernel` is
in it. After the 2026-09-28 purge it is 11 107 LOC with 11 responsibilities and
**zero dependents** — nothing in the workspace links to it, because the kernel
binary's own `main.rs` is a compilation root, not a Cargo dependent. By the
ratified `justification_rule` it is unjustified. That is not absurd; it is the
honest reading: the kernel has no caller. It becomes justified the moment a real
service consumes it (the planned `cynic-api` would), or it becomes a pure
interface crate. **Both are decisions; neither should happen by accident.**

### The bound, and why it is the one number that is a convention

The data is bimodal: 2 crates have one responsibility, 16 have more than 8.
A "responsibilities ≤ N" threshold is therefore easy to place and is the only
number in this document that is **chosen rather than derived**. It is recorded as
a shrink-only baseline so that it can only tighten. Two honest framings, and the
operator picks one:

- record the **bimodality** as the invariant ("no crate between 1 and 8
  responsibilities"), or
- fix **8** as an anchor and let the measured distribution argue over it.

What is *not* a choice: a crate that grows past its recorded responsibility
count fails, exactly as a claim that grows past its recorded size fails today.

## 6. Where each rule is executed

A rule that is not executed is a wish. Every row below is machine-checked.

| rule | executed by | schema |
|---|---|---|
| reachability (what is compiled) | `crates/*/tests/orphan_sources_ratchet.rs` + shared detector | fails both directions |
| crate justification | `infra/crate-topology.json` → `justification_rule` | `cynic.crate_topology.v1` |
| layer direction | `cynic dev crate-audit` → `entrypoints_bounded`, `architecture_boundary_test_present` | `cynic.cli.crate_audit.v1` |
| domain layers / cross-family edges | `tests/architecture_boundary.rs` per crate | — |
| branch liveness, worktree/claim cross-check | `cynic dev branch-audit` | `cynic.cli.branch_audit.v1`, `cynic.cli.worktree_observation.v1` |
| secret + path + node-host audits | `cynic audit {secret,paths,node-host}` | `cynic.node_host_audit.v1` |

Two facts about the current instruments, recorded so nobody trusts them by
accident:

- **`cynic dev crate-audit` does not measure what its name implies.** Its four
  checks are file-shape checks (`entrypoints_bounded` at 300 lines for
  `main.rs`, 220 for `lib.rs`; `hotspots_observed` at 400 lines per file). It
  measures no responsibility, no dependency and no dead mass. The name promises
  a boundary; the implementation counts lines. It is the last place where the
  line-count reflex is still hard-coded, and it should lose those constants
  rather than gain more.
- **Two entry points, one in the gate.** `cynic audit {crate,secret,paths,node-host,all}`
  is what `make check` runs. `cynic dev {crate-audit,kernel-audit,ownership-audit,
  surface-audit,branch-audit}` is not. The same subject can be checked in two
  places with two report shapes, and only one of them is load-bearing.

## 7. The order of work, and why

The dependency order is not by size or by count. It is by what unblocks the rest.

1. **Canonical vocabulary first** (`cynic-protocol`, 106 responsibilities, 35
   dependents). Every other decision about `Claim`, `Scope`, `Budget` and
   `ClaimStatus` is undecidable while five homes exist. Extracting anything
   before this just moves the ambiguity.
2. **`FRACTURE` downward** — `cynic-host` (124), then `cynic-node` (59), then
   `cynic-cli` (86), each by named responsibility, in the direction their
   families already imply (`cynic-host-*`, `cynic-node-*`).
3. **`PARKED` mechanically** — verdicts already written in
   `infra/CRATE_FUSION_DISPOSITIONS.md`; read before acting, they are ratified.
4. **`UNJUSTIFIED` as a decision** — `cynic-kernel`, `cynic-mcp`,
   `cynic-observer`: each needs either a real consumer or a recorded retirement.
5. **Emergent crates last, with their reason written first**: `cynic-api` is
   justified by `crate_justified` condition 2 (a distinct binary/service
   requirement) and condition 5 (the kernel is forbidden that surface, locked by
   test). It is not a new layer — it is a composition root for the runtime,
   where the operator-facing composition root is today `cynic-cli` (which is why
   it imports 19 crates, and why its name under-describes the job).

## 8. What is deliberately not decided here

- The responsibility bound (see §5) — a convention, and the operator's call.
- Whether `cynic-core` (the hexagonal application/composition layer proposed in
  `docs/archive/reports/overlap-analysis.md`, and implemented in the archived
  `vague-b-eval` branch) deserves a name of its own, or whether the composition
  role stays split between `cynic-cli` (operator) and `cynic-api` (runtime).
  The branch that created it also violated the layer rule, so it is parked, not
  rejected. The question is open because the answer changes what `cynic-cli` is.
- Whether line counts are ever restored as a guard. Recommended: keep the ceiling
  as blast-radius only, never as a design rule, because `SURCHARGE = 0`.
