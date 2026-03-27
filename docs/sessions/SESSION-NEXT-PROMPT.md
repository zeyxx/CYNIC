# Next Session — v0.8 "Guérir l'Organisme" (continued)

*Written 2026-03-27 post-deploy. Commit 82f0a7c on main.*

## What Was Done (verified, deployed, running)

Crystal lifecycle integrity — 6 boundary enforcements, 7 findings closed, 281 tests.
Read: `docs/architecture/CYNIC-ARCHITECTURAL-TRUTHS-V08.md` (T1-T17, all falsified)
Tracker: `docs/audit/CYNIC-FINDINGS-TRACKER.md` (honest statuses)
Session learnings: `.claude/projects/*/memory/project_v08_wave01.md`

## What's Missing (honest, prioritized by compound value)

### Priority 1 — Testing Infrastructure (NO CI/CD EXISTS)

**Problem:** 281 tests pass locally but there is no CI pipeline. No E2E tests against the running kernel. No user-facing test suite. The deploy step is manual `systemctl stop/cp/start`. This is the biggest gap for a 7-year foundation — code without CI rots.

**Research needed before acting:**
- What's the simplest CI for a single-binary Rust project on a Tailscale network?
- GitHub Actions can run `cargo test` but can't reach SurrealDB or sovereign Dogs
- E2E tests need the kernel running — how to structure?
- `/test-chess` skill exists but is manual, not automated

**Protocol:**
1. `/cynic-skills:cynic-empirical` — research CI patterns for Rust + SurrealDB + sovereign infra
2. `/cynic-skills:crystallize-truth` — decide: GitHub Actions for unit tests + local script for integration + systemd timer for E2E?
3. Implement the SIMPLEST thing that catches regressions before deploy

### Priority 2 — Straightforward Fixes (6 findings, no research needed)

| Finding | Fix | Estimated complexity |
|---|---|---|
| F2 | X-Forwarded-For → axum ConnectInfo | 1 file (middleware) |
| F13 | content.chars().count() instead of .len() | 1 line |
| F22 | /ready cache DB ping result (30s TTL) | ~20 lines |
| F23 | /events SSE connection limit | ~10 lines |
| F6 | gemma parse failure (prompt format) | research Dog prompt |
| RC1-6 | Event injection input validation | ~10 lines |

**Protocol:** Fix → Test → `/build` → verify. One commit per fix. No batching.

### Priority 3 — StoragePort Trait Split (architectural cleanup)

Split `StoragePort` (25 methods, 12 silent defaults) into per-aggregate traits:
`VerdictStore`, `CrystalStore`, `ObservationStore`, `SessionStore`, `UsageStore`, `MetaStore`.

**Protocol:**
1. `/cynic-skills:cynic-empirical` — review howtocodeit/hexarch and Microsoft Rust template for concrete patterns
2. Design the split (sub-trait signatures)
3. Implement with contract tests per sub-trait
4. Verify all 281+ tests still pass

### Priority 4 — DeterministicDog Quality (research gate)

F9/F10/F11 — DeterministicDog heuristics produce false positives. With MIN_QUORUM=2, it's always 50% of minimum consensus. Its biases leak into every crystal.

**Protocol:**
1. Query production verdicts where deterministic-dog disagreed with LLM Dogs
2. Analyze which patterns cause false positives
3. Only then improve heuristics (data-driven, not guessing)

### v0.9 Scope (architectural, not v0.8)

- ChatPort multi-turn → structural stimulus isolation (StruQ USENIX Sec'25)
- RC7 observability → request_id propagation through PipelineDeps
- StoragePort trait split (if not done in v0.8)

## Session Protocol

```
BEFORE any work:
  1. /status — verify kernel is running, Dogs healthy
  2. cargo test --quiet — verify baseline (281+ tests)
  3. Read this document + CYNIC-FINDINGS-TRACKER.md

DURING work:
  4. Rule 36: bugs before abstractions
  5. Rule 35: gate at lowest common caller
  6. Rule 25: Fix → Test → Gate → Verify
  7. /build after ANY kernel code change
  8. One commit per logical change (not batched)

BEFORE claiming done:
  9. Adversarial review (superpowers:code-reviewer)
  10. Only claim what's PROVEN by tests, not what looks correct
  11. Update tracker with HONEST status

AT session end:
  12. /cynic-skills:distill
  13. git status --short must show 0 modified files
```

## Rules to Follow

- Rule 1: Diagnose before fixing
- Rule 25: Fix → Test → Gate → Verify
- Rule 31: Measure before AND after
- Rule 34: Falsify before adopting
- Rule 35: Gate at lowest common caller
- Rule 36: Bugs before abstractions

## References

- Architectural truths: `docs/architecture/CYNIC-ARCHITECTURAL-TRUTHS-V08.md`
- Findings tracker: `docs/audit/CYNIC-FINDINGS-TRACKER.md`
- Session memory: `memory/project_v08_wave01.md`
- Deep audit: `docs/audit/CYNIC-DEEP-AUDIT-2026-03-25.md`
- Research: arxiv 2504.18333 (LLM-as-judge), StruQ USENIX Sec'25, OWASP LLM01:2025
