# CCM — Compound Cognitive Memory

*How sessions compound learning over time without creating debt.*

## Evidence

39 sessions. 101 memory files. 3740 lines. ~400 findings. 24 CLAUDE.md rules.
300+ findings sit in memory files at ~10% efficacy — recalled occasionally, enforced never.

**The gap is not documentation. It's enforcement.**

## The Enforcement Pyramid

| Level | Mechanism | Efficacy | Debt | Evidence |
|-------|-----------|----------|------|----------|
| L4 | Compiler / type system | ~100% | 0 | `#![deny(dead_code)]` catches every dead symbol, every build |
| L3 | Hook / gate | ~80% | Low | `protect-files.sh` deny, pre-push gitleaks — bypassed only deliberately |
| L2 | CLAUDE.md rule | ~50% | 0 | Rule #12 "fix the class" — followed in most sessions, violated in some |
| L1 | Memory file | ~10% | ~0 | `feedback_diagnose_deep.md` — helps only when recalled at the right moment |
| L0 | Undocumented | 0% | 0 | Lost knowledge |

**Compound formula**: `value = Σ(pattern_i × efficacy_i)`

One pattern at L4 (compiler) = 10 patterns at L1 (memory).
The protocol's job: **move patterns UP the pyramid.**

## What Compounds (Measured)

1. **Compiler-enforced rules** — `#![deny(dead_code, unused_imports)]` has caught more bugs than any session of review. Zero maintenance. Infinite duration.
2. **Hooks** — `protect-files.sh` prevented zone violations after one setup. `gitleaks` prevented secret leaks after one commit.
3. **CLAUDE.md rules** — 24 rules accelerate every session post-writing. Rule #18 (integration tests for SQL) found bugs on first run, three sessions in a row.
4. **Git commit discipline** — every message is compound at zero cost. `git log --grep` is free search.

## What Does NOT Compound (Measured)

| Mechanism | Result | Evidence |
|-----------|--------|----------|
| Crystal DB | 0 mature crystals in 2 weeks | 2115 observations, 20 crystals all Decaying |
| Background tasks | 4/5 never completed a useful cycle | coord_expiry, usage_flush, ccm_aggregate, summarizer |
| New API endpoints | Each adds auth + rate limiting + testing surface | 14 endpoints, each is maintenance |
| Aspirational architecture | Dead code, confident names, weeks of building on false foundations | Crystal loop, temporal perspectives, sovereign "live" at 52-70% failure |

**Rule: if it needs a server UP to compound, it's debt, not compound.**

## The Distillation Loop

```
session experience
  → /distill (harvest → extract → search → promote → curate)
  → finding classified:
      NEW (0 prior)        → memory file       (L1, ~10%)
      RECURRING (2+ prior) → CLAUDE.md rule     (L2, ~50%)
      ENFORCEABLE          → compiler/hook      (L3-L4, 80-100%)
  → CLAUDE.md curated (merge overlaps, burn obsoletes)
  → next session starts with higher enforcement baseline
```

### Inputs (all exist, zero new infra)

1. **Session context** — what was built, what failed, what surprised
2. **Memory files** — 101 files, recurrence detection via grep
3. **CLAUDE.md** — 24 rules, the enforcement layer
4. **Git history** — every commit message, free forever
5. **Observations API** — optional enrichment via `GET /crystals` (hot files, co-edits)

### When to Distill

- After significant session work (committed code, fixed bugs, completed features)
- After an incident (production bug, multi-hour debugging, data loss)
- Before ending a long session (>2h of focused work)
- **NOT**: after trivial changes, status checks, exploration-only sessions

### Promotion Criteria

**L1 → L2 (Memory → CLAUDE.md):**
- Pattern appeared in 2+ separate sessions (grep memory files), OR
- Pattern caused >1h wasted in a single session, OR
- Pattern is a CLASS (not an instance)

**L2 → L3/L4 (CLAUDE.md → Compiler/Hook):**
- The rule can be checked mechanically (grep, clippy, type system)
- The rule has been violated despite being in CLAUDE.md
- The cost of violation is high (security, data loss, multi-hour waste)

### Curation

CLAUDE.md has a carrying capacity of ~50 rules. Beyond that, rules conflict and get ignored.

- **Merge**: two rules that say similar things → one precise rule
- **Burn**: rules now enforced by compiler/hooks → remove from CLAUDE.md (the compiler IS the rule)
- **Rewrite**: vague rules → specific, falsifiable, grep-checkable
- **Current**: 24 rules. Capacity: ~50. Headroom: ~26 slots.

## The 10 Recurring Patterns (Corpus Analysis, March 2026)

These patterns appeared in 3+ sessions despite documentation. Each represents compound value waiting to be promoted:

| # | Pattern | Sessions | Current Level | Target |
|---|---------|----------|---------------|--------|
| 1 | Fix instance, not class | 8+ | L2 (Rule #12) | L3 (hook: grep sweep gate) |
| 2 | Config/secret drift | 4+ | L2 (Rule #14) | L3 (hook: env diff check) |
| 3 | Dead architecture shipped as alive | 5+ | L2 (Rules #21, #24) | L2 (hard to automate) |
| 4 | SQL bugs only found at runtime | 4+ | L3 (Rule #18 + make check-storage) | L3 (achieved) |
| 5 | Sovereign inference broken silently | 3+ | L2 (Rule #23) | L3 (/test-chess gate) |
| 6 | Multi-agent file collisions | 3+ | L2 (coord) | L3 (worktrees) |
| 7 | Skills ignored for manual commands | pervasive | L1 (feedback file) | L2 (CLAUDE.md) |
| 8 | Fix-and-shift (53% of fixes create new bug) | pervasive | L2 (Rule #12) | L4 (compiler: consumer check) |
| 9 | Hardcoded values | 5+ | L2 (Rule #14) | L3 (grep hook) |
| 10 | Claude makes false assertions | 3+ | L1 (feedback file) | L2 (verification skill) |

## Relationship to Crystal System

Two independent loops:

- **Loop A (Judge)**: crystals → Dog prompts → better scoring → updated crystals. Bottleneck: exact-match crystal IDs prevent accumulation. Needs semantic clustering. Engineering problem, lower priority.
- **Loop B (Developer)**: session experience → distilled rules → better coding → session experience. Bottleneck: no distillation ritual. **Solved by this protocol.**

Loop B compounds TODAY with zero code changes.
Loop A requires kernel engineering (semantic clustering). Separate concern.

## Metrics

1. **Enforcement coverage**: X/10 known patterns at L2+ (target: 10/10)
2. **Compound rate**: rules promoted per session (expected: 0-2, log curve)
3. **CLAUDE.md health**: rule count / ~50 capacity
4. **Curation ratio**: rules merged or burned (>0 when count > 40)

## The Compound Invariant

After every session: `enforcement_level(now) >= enforcement_level(before)`

Diminishing returns are correct — that's log(n). Each session adds less.
But NEVER subtracts. That's the invariant.

---

*Crystallized 2026-03-22. Evidence: 39 sessions, 101 memory files, 10 recurring patterns, 0 mature crystals.*
