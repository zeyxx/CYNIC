# CYNIC Agent Workflow Design

**Date:** 2026-03-28
**Status:** DESIGN COMPLETE, NOT IMPLEMENTED
**Scope:** v0.8 (Layer 0-1), v0.9 (Layer 2-3)

## Problem Statement

Git history analysis (304 commits, 18 days) reveals three root causes of a 1.30 fix/feat ratio:

1. **Research-after-adoption** — features shipped before production behavior understood (gRPC, MCTS Temporal, SurrealDB 3.x, backtick IDs). Rule 34 was written after paying for all four.
2. **Audit-driven bug discovery** — bugs accumulate until batch audit (14 bugs in one compound commit), not caught inline. Test commits = 2.6% of history.
3. **Diagnosed-but-never-fixed** — Chain 2, event bus consumers, DeterministicDog F9/F10/F11, crystal positive-only bias. All root-caused, zero code changes.

The workflow relies on CLAUDE.md text reminders — "hope engineering." No mechanical enforcement exists beyond `make lint-rules` and pre-push hooks.

## Design Principles

- Deterministic gates before heuristic agents (grep before LLM)
- Agents use the platform, not their own state (Rule 28)
- Batch audit over per-edit dispatch (cost-bounded)
- Sovereign inference for self-audit (CYNIC judges CYNIC)
- No new ports/traits until contract tests pass

## Architecture: 4 Layers

### Layer 0: Deterministic Gates (v0.8, immediate)

New Makefile target `lint-drift`, added to `make check` (already in pre-push hook).

**Checks (all grep/parse-based, 0 tokens, <2s):**

| Check | Method | What it catches |
|---|---|---|
| Dog name alignment | Parse `backends.toml`, verify model field matches reality | Dog names that lie (qwen3-4b-ubuntu runs gemma-3-4b) |
| Module liveness | Grep `^// pub mod` in `domain/mod.rs`. Each match must have `// DORMANT: <reason>` on the same line (e.g., `// pub mod temporal; // DORMANT: fake wiring burned 2026-03-24`). Pattern: `grep -E '^\s*//\s*pub mod' \| grep -v 'DORMANT:'` = violation. | Dead modules on disk without documented reason |
| Skill table sync | Extract local skills (no `:` namespace) from CLAUDE.md, check `.claude/commands/<name>.md` exists. Skip namespaced plugin skills (`cynic-skills:*`, `superpowers:*`). | Phantom local skills in the routing table |
| Untracked hooks | Compare `.claude/hooks/*.sh` against `settings.local.json` hook entries | Hooks on disk but not wired |
| Producer-consumer (Rule 33) | Grep `store_*` in StoragePort, verify matching `list_*`/`get_*` | Store paths without read paths |

**Wiring:** `make check` calls `lint-drift` after `lint-rules`. Pre-push hook gates automatically. Zero new infrastructure.

### Layer 1a: kernel-auditor Batch Dispatch (v0.8)

The `.claude/agents/kernel-auditor.md` subagent already exists (Read/Grep/Glob only, model=sonnet). Currently invoked manually.

**Changes:**
- Wire as a **second entry** in the `Stop` hook array in `settings.local.json` (after `session-stop.sh`). Both entries must be independently fail-safe (`|| true` semantics) — one failure must not prevent the other.
- `session-init.sh` writes a timestamp file (`/tmp/cynic-session-start-$SESSION_ID`) at session start. The auditor uses this to scope `git diff --name-only` to files modified during the session.
- Rules checked: 8 (fallible I/O), 17 (adapter through port), 19 (no logic duplication), 22 (no trait name collision), 32 (no cross-layer leakage), 33 (producer-consumer)
- Output: findings written to `.claude/agent-memory/audit-<date>.md`
- Cost: ~5K tokens, 1x per session (not per edit)

**Why batch, not per-edit:** The protect-files.sh hook already adds 2-3s latency per kernel edit (2 HTTP calls). Adding an LLM dispatch per edit would make editing unusable. Batch at session end catches the same issues without impacting flow.

### Layer 1b: Sovereign Self-Audit (v0.8)

New background task in `infra/tasks.rs`: `spawn_self_audit`.

**Mechanism:**
- Ticks every 6 hours (configurable via env)
- Constructs an alignment-check prompt containing:
  - Active rules (from CLAUDE.md + .claude/rules/, <2K tokens)
  - Public interfaces (`pub fn`, `pub trait`, `pub struct` — extracted via grep, <3K tokens)
  - Open findings summary (<1K tokens)
  - Recent crystals domain=cynic-internal (<1K tokens)
- Sends to sovereign LLM via `Arc<dyn InferPort>` (not concrete `SovereignSummarizer`). Note: the existing summarizer task takes a concrete struct — self-audit should take the trait to enable mock testing per Rule 17. Requires wiring `Arc<dyn InferPort>` in `main.rs`.
- Parses response into alignment findings
- Stores as crystals with `domain = "cynic-internal"`, `state = Forming`, `voter_count = 0`
- **Quorum exception:** Internal observations (domain=`cynic-internal`) bypass the MIN_QUORUM gate. The quorum gate in StoragePort must check `domain != "cynic-internal"` before rejecting. This is a policy exception, not a workaround — internal self-observations are structurally different from external epistemic judgments.
- Findings visible at `GET /crystals?domain=cynic-internal`

**Constraints:**
- Qwen 3.5 9B context: ~32K tokens. Prompt must stay under 8K to leave room for response.
- No full repo dump — interfaces + rules + findings only.
- Graceful degradation: if sovereign LLM unreachable, skip silently (same pattern as summarizer).

### Layer 2: Skill Router (post Layer 0-1, depends on embedding infra)

Implements the existing design in `docs/architecture/SKILL-ROUTER-DESIGN.md`.

**Summary:**
- `UserPromptSubmit` hook in `settings.local.json`
- Qwen3-Embedding-0.6B deployed at `:8081` on sovereign infra
- 67 pre-computed skill vectors in `.claude/data/skill-vectors.json`
- Cosine similarity: if score > 0.5 AND gap > 0.05, inject skill suggestion into `additionalContext`
- 98ms total latency budget (84ms embed + 14ms cosine)
- Feedback loop: `/observe` captures whether the suggested skill was actually invoked

**Prerequisites:**
- Qwen3-Embedding-0.6B deployed and health-checked on sovereign infra
- Fix hardcoded IP in `SKILL-ROUTER-DESIGN.md` line 37 → use `${CYNIC_EMBED_HOST}` from `~/.cynic-env`
- Validate accuracy on real session transcripts (92 `.jsonl` files) before shipping — the 90% benchmark is biased (same author wrote examples and tests, per SKILL-ROUTER-DESIGN.md acknowledgment)

### Layer 3: Kernel-Native MAPE-K Consumers (v0.9)

Tokio tasks in `infra/tasks.rs`, same pattern as the 7 existing background tasks.

| Event | Consumer | Action |
|---|---|---|
| `DogFailed` | Circuit breaker | 3 failures in 5min → disable Dog for 15min, emit Anomaly |
| `Anomaly` | Alert escalator | `tracing::warn` + increment metric + crystal domain=cynic-internal |
| `CrystalObserved` | Drift detector | Confidence decaying faster than phi^-2 → flag |
| `VerdictIssued` | Quality monitor | Q-score trend tracking, alert on sustained decline |

These consumers close the "event bus 0 internal consumers" gap that has been diagnosed since 2026-03-24.

## Sequencing

```
v0.8 immediate:
  1. make lint-drift (Layer 0)
  2. kernel-auditor Stop hook (Layer 1a)
  3. spawn_self_audit background task (Layer 1b)

v0.8 after embedding infra:
  4. Skill router (Layer 2)

v0.9:
  5. MAPE-K consumers (Layer 3)
```

## What We Explicitly Do NOT Build

- **Per-edit agent dispatch** — too expensive, adds latency, fragile
- **Agent-owned databases** — Rule 28, agents use the kernel's storage
- **LLM-based drift detection** — grep suffices for names, modules, skills
- **New port traits** — not until contract tests validate the existing ones
- **Manual personas/modes** — too much friction, doesn't compound

## Success Metrics

| Metric | Before | Target |
|---|---|---|
| Fix/feat ratio | 1.30 | < 0.7 (measured: `git log --pretty="%s" \| grep -c "^fix:"` vs `grep -c "^feat:"` over next 50 commits) |
| Drift detection | manual audit | mechanical, every push |
| Rules violation detection | batch audit sessions | per-session + continuous |
| Self-audit coverage | 0 | sovereign LLM checks every 6h |
| Event bus internal consumers | 0 | 4 (v0.9) |

## Dependencies

- Layer 0: None (Makefile + grep)
- Layer 1a: None (agent exists, hook wiring only)
- Layer 1b: Sovereign LLM reachable, InferPort live (already true)
- Layer 2: Qwen3-Embedding-0.6B deployed at :8081
- Layer 3: None — existing `tokio::sync::broadcast` already supports multiple `.subscribe()` calls. No refactor needed.
