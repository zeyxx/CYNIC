# Workflow Truth Audit — 2026-04-15

## Purpose

This audit separates:

- stable protocol
- agent-specific adapters
- volatile session journal
- roadmap / backlog
- actual mechanical enforcement

The repo currently contains all five layers, but some files still mix them.

## Scope

Audited sources:

- `AGENTS.md`
- `CLAUDE.md`
- `GEMINI.md`
- `.codex/CODEX-PROTOCOL.md`
- `.handoff.md`
- `TODO-ROBUSTNESS.md`
- `TODO.md`
- `docs/TODO.md`
- `.claude/rules/workflow.md`
- `scripts/git-hooks/pre-commit`
- `scripts/git-hooks/pre-push`
- `.claude/hooks/session-init.sh`
- `.claude/hooks/session-stop.sh`
- `.claude/hooks/coord-claim.sh`
- `scripts/coord-claim-gemini.sh`
- `.codex/config.toml`
- `.claude/settings.json`
- `.gemini/settings.json`

## Executive Truths

| T# | Truth | Confidence | Design impact |
|----|-------|------------|---------------|
| T1 | `AGENTS.md` is the best candidate for the repo-wide protocol source of truth. | 0.61 | Keep shared lifecycle, claims, handoff semantics, and security rules there. |
| T1a | `.codex/CODEX-PROTOCOL.md` is not currently an adapter only; it mixes protocol, roadmap, and stale session state. | 0.58 | Reduce it to Codex-specific execution notes and fallback mechanics. |
| T2 | `.handoff.md` is a historical journal, not a normative document. It contains superseded truths by design. | 0.61 | Explicitly mark it as non-normative and require “superseded by” semantics when needed. |
| T3 | Claude has real session automation today; Gemini has partial file-claim automation; Codex still depends on manual or fallback execution. | 0.60 | Document the lowest common executable workflow, not the richest client-specific workflow. |
| T4 | `TODO-ROBUSTNESS.md` and `docs/TODO.md` currently preserve stale reality snapshots. | 0.56 | Re-scope one as roadmap and one as session protocol, or archive stale sections. |
| T5 | The repo's largest workflow problem is not lack of docs, but role confusion between spec, journal, and backlog. | 0.61 | Enforce a strict document-role hierarchy before continuing large implementation work. |

## Document Role Matrix

| File | Current role | Should be normative? | Status | Main issue |
|------|--------------|----------------------|--------|------------|
| `AGENTS.md` | Shared multi-agent protocol | Yes | Mostly healthy | Also serves as Codex entrypoint, which is acceptable if kept stable. |
| `CLAUDE.md` | Claude constitution + canonical philosophy + build/security guidance | Yes, for Claude-specific and repo philosophy | Healthy | Very broad; overlaps with workflow docs but remains canonical for Claude. |
| `GEMINI.md` | Gemini adapter + operational notes | Yes, Gemini-only | Partially healthy | Contains assumptions about tools/config that may drift from actual `.gemini/settings.json`. |
| `.codex/CODEX-PROTOCOL.md` | Codex adapter + roadmap + stale state | No, not as currently written | Unhealthy | Mixes enduring protocol with completed TODOs and historical cleanup state. |
| `.handoff.md` | Append-only session journal | No | Healthy as a log, unhealthy as perceived truth source | Needs explicit non-normative framing and supersession discipline. |
| `TODO-ROBUSTNESS.md` | Sprint roadmap | No | Stale | Contains already-completed work as active instructions. |
| `TODO.md` | Session protocol + active work ledger | Partially | Mixed | Calls itself “protocol, not backlog”, but also contains mutable backlog and historical notes. |
| `docs/TODO.md` | Open work ledger | No | Stale | Duplicates TODO semantics and preserves obsolete debt wording. |
| `.claude/rules/workflow.md` | Operational discipline for Claude | Yes, but Claude-scoped | Healthy | Must not be mistaken for universal automation across all agents. |

## Mechanization Matrix

| Capability | Claude | Gemini | Codex | Truth |
|-----------|--------|--------|-------|-------|
| Session start automation | Yes (`session-init.sh`) | No repo-proven equivalent | No repo-proven equivalent | Only Claude currently automates `register + probe + inject context`. |
| Session end automation | Yes (`session-stop.sh`) | No repo-proven equivalent | No repo-proven equivalent | Only Claude currently automates release/compliance/session summary. |
| File claim before write | Yes (`coord-claim.sh`) | Partial (`BeforeTool` + helper script) | No confirmed hook path | Shared protocol exists; automation parity does not. |
| Handoff read/write via MCP | Yes, via kernel/MCP path | Plausible, partially documented | Configured, but session reality varies | Must document fallback path explicitly. |
| Build/test gate enforcement | Yes (`pre-commit`, `pre-push`) | Indirect via git hooks | Indirect via git hooks | These are repo-wide and reliable. |

## Contradictions And Drift

### 1. `AGENTS.md` vs `.codex/CODEX-PROTOCOL.md`

Observed:

- `AGENTS.md` presents shared protocol and Codex constraints.
- `.codex/CODEX-PROTOCOL.md` repeats protocol but also embeds:
  - completed TODO #3 state
  - active TODO #4 recommendations
  - assumptions about auto-claim hooks for Codex

Why this is a problem:

- adapter docs should explain execution of the shared protocol
- they should not become a second evolving source of repo state

### 2. `.handoff.md` vs stable docs

Observed:

- handoff entries mention states that were true at write time but are now stale:
  - old tree state
  - old stack values
  - old “next” items
  - old unfinished claims about Codex wiring

Why this is not inherently wrong:

- historical logs SHOULD preserve what was believed at that time

Why it becomes harmful:

- when later sessions read it as if it were current instruction rather than journal

### 3. `TODO-ROBUSTNESS.md` and `docs/TODO.md`

Observed:

- both still contain obsolete work as if it were pending
- both duplicate roadmap/backlog semantics

Impact:

- a new session can pick up already-closed work and regress into duplicate effort

### 4. Declared automation vs real automation

Observed:

- Claude has concrete hook-backed automation.
- Gemini has a concrete claim helper and config, but not the full same lifecycle automation.
- Codex has config and repo protocol, but not the same deterministic lifecycle automation.

Impact:

- any shared protocol that says “all agents do X automatically” is false

## Canonical Hierarchy Proposal

### Layer A — Stable protocol

- `AGENTS.md`

Contains:

- repo-wide multi-agent lifecycle
- claims / conflicts / release rules
- handoff semantics
- security invariants
- lowest-common-denominator fallback path

### Layer B — Agent adapters

- `CLAUDE.md`
- `GEMINI.md`
- `.codex/CODEX-PROTOCOL.md`

Contain only:

- how that client executes Layer A
- client-specific strengths/limits
- real automation available in that client
- fallback path when automation is missing

Must not contain:

- sprint state
- temporary cleanup lists
- global “next” roadmap

### Layer C — Volatile journal

- `.handoff.md`

Contains:

- dated session summaries
- what changed
- what was learned
- what is blocked
- what supersedes earlier assumptions

Must not be treated as protocol.

### Layer D — Roadmap / backlog

- `TODO.md` or `TODO-ROBUSTNESS.md`, but not both as live authorities

Recommendation:

- keep one active roadmap
- archive or collapse the other

## Falsifiable Checks

The hierarchy is correct only if these checks pass:

1. A new Codex session can determine the correct workflow by reading `AGENTS.md` plus `.codex/CODEX-PROTOCOL.md`, without consulting `.handoff.md` for normative behavior.
2. A stale `.handoff.md` entry cannot mislead an agent about current policy because stable docs explicitly outrank it.
3. A completed roadmap item appears in at most one place as “done”, and in zero places as “still active”.
4. Every claim of automation can be pointed to a real hook, script, or config file.

## Immediate Cleanup Sequence

1. Reconcile `AGENTS.md` and `.codex/CODEX-PROTOCOL.md`
   - remove roadmap/state from Codex adapter
   - keep only Codex-specific execution details
2. Add an explicit non-normative banner to `.handoff.md`
   - historical journal
   - may contain superseded entries
3. Collapse roadmap authority
   - choose one live roadmap file
   - archive or rewrite stale TODO documents
4. Write the lowest-common-denominator workflow explicitly
   - `register -> read handoff -> who -> claim -> work -> append -> release`
   - “automatic where available, manual otherwise”

## Hostile Review

Strong objection:

"This is over-documenting a problem that should be solved by hooks."

Why the objection does not fully win:

- hooks are not presently symmetric across clients
- repo governance still needs one stable human-readable source of truth
- without hierarchy, even perfect hooks do not prevent conceptual drift

Stronger counter-risk:

- if the repo keeps adding protocol docs without deleting stale ones, the audit itself becomes another layer of drift

Mitigation:

- after reconciliation, either archive this audit or reference it from one canonical location only

## Decision

Do not start K15 or further organism wiring until the protocol layers are reconciled.

The next correct commit is documentation/governance work:

- reconcile stable protocol vs adapter vs journal vs backlog
- then enforce the result mechanically
