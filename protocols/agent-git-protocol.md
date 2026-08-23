# CYNIC Agent Git Protocol — protocols/agent-git-protocol.md

## Purpose and Authority

This document is the **normative Git protocol** for any agent that mutates
CYNIC source. It was previously cited by `agent-protocol.md` but did not
exist; this file closes that gap.

**Authority:** This prose is a projection. The executable law lives in Rust:

- `crates/cynic-protocol/src/source_policy.rs` — pure predicates
  (`commit_conforms`, `branch_conforms`, `ref_is_protected`, `agent_branch_for`)
  + `CommitPolicy` / `BranchPolicy` types. Source of truth for the rules below.
- `crates/cynic-protocol/src/source_control.rs` — `admit_source_change` (blocks
  non-conventional commit intents) and `admit_source_stage(SovereignCommit)`
  (refuses promotion to protected trunks and non-conforming branches).
- `crates/cynic-protocol/src/agent_citizenship.rs` — the admission gate that
  decides *whether an agent may mutate at all* (claim + budget + store).
- `infra/agent-source-protocol.json` — the machine-readable contract
  (`cynic.agent_source_protocol.v1`) that mirrors these rules declaratively.

Where this document disagrees with those Rust types, **the Rust wins** until a
migration lands.

## The Four Rules

### Rule 1 — Conventional Commits
Every commit intent must match `<type>(<scope>): <subject>` (breaking:
`<type>(<scope>)!: <subject>`). Accepted types:
`feat, fix, test, docs, refactor, chore, ci, build, perf, style`.
Enforced by `admit_source_change` via `commit_conforms`.

### Rule 2 — Branch isolation
An agent works on `agent/<agent_id>/<scope>` (or one of the other accepted
prefixes: `integration`, `recovery`, `backup`, `feat`, `fix`, `docs`,
`refactor`, `test`, `chore`). Enforced by `branch_conforms`.

### Rule 3 — No direct push to protected refs
`main`, `forgejo/main`, `origin/main`, `master` are protected. An agent may
never push to them directly; promotion happens only through a merge bot / PR
flow. Enforced by `admit_source_stage(SovereignCommit)` +
`ref_is_protected` (emits `sovereign_promotion_to_protected_ref`).

### Rule 4 — One agent, one worktree
An agent mutates source only inside an isolated `git worktree` on its own
branch. Never mutate a checked-out trunk in the primary checkout.

## Topology (observe, do not hardcode)

- **Forgejo** (`forgejo.synapse.cynic`, 10.10.10.101) is the **sovereign SSOT**.
  Canonical push URL: `ssh://forgejo@forgejo.synapse.cynic:22/jeanterre13/CYNIC.git`.
- **Builder** (10.10.10.119, `cynic-builder.synapse.cynic`) is an LXC that
  **mirrors** the SSOT and runs the build. It is **not** the source of truth.
  The cortex local remote currently points at the builder by mistake; it must
  point at the Forgejo SSOT.
- **GitHub** (`origin`) is a public relay/mirror only. Force-push mirror is
  acceptable; never create PRs here.
- Reconcile order: `cortex → forgejo → github`.

## Citizenship → Source (the link)

An agent may propose a source change only after passing
`agent_citizenship::admit_mutation` (active claim + budget + store available).
The source policy (`source_policy.rs`) then validates *how* the change is
expressed (commit/branch/ref). The two gates are sequential, not duplicated:
citizenship answers "may you mutate?", source policy answers "is the change
well-formed?".

## Proof

A source change is proved by `crates/cynic-host/src/source_control.rs`
(`GitSourceControlAdapter`): worktree lock, fencing token, base-commit check,
`ls-remote` probe, push, and a read-back `ls-remote` proving the remote head
matches the local head. A command that returns success without read-back is
not proof.

*This document is prose. Law lives in Rust. Transpose, compile, test.*
