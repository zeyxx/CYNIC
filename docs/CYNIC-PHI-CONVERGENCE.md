# CYNIC — Phi-Convergence: Confidence Beyond φ⁻¹

**Date:** 2026-03-17
**Method:** Crystallize Truth (3-layer) + mathematical proof
**Status:** Crystallized — proved analytically, not yet implemented in code.
**Epistemic stance:** This document itself is bounded at φ⁻¹. The math is exact; the interpretation carries uncertainty.

---

## The Discovery

CYNIC bounds all confidence at φ⁻¹ = 0.618 (61.8%). This is correct for a **single measurement event** — one evaluation, however many Dogs participate.

But independent convergence over time genuinely increases confidence. Bayesian inference, signal averaging, scientific replication — all confirm this. The question: can confidence grow beyond φ⁻¹ while remaining φ-derived?

**Answer: yes.** The formula is:

```
Max confidence after n levels of independent convergence:

    C(n) = 1 - φ⁻⁽²ⁿ⁺²⁾

    C(0) = 1 - φ⁻² = φ⁻¹           = 0.618034  (61.8%)
    C(1) = 1 - φ⁻⁴                  = 0.854102  (85.4%)
    C(2) = 1 - φ⁻⁶                  = 0.944272  (94.4%)
    C(3) = 1 - φ⁻⁸                  = 0.978714  (97.9%)
    C(∞) = 1 - 0                     → 1.0       (unreachable)
```

### Proof

The partial sums of odd powers of φ⁻¹ satisfy:

```
Σ(k=0..n) φ⁻⁽²ᵏ⁺¹⁾ = 1 - φ⁻⁽²ⁿ⁺²⁾
```

This is a geometric series with ratio φ⁻²:

```
Σ(k=0..∞) φ⁻⁽²ᵏ⁺¹⁾ = φ⁻¹ / (1 - φ⁻²) = φ⁻¹ / φ⁻¹ = 1.0  exactly
```

Each level of convergence adds φ⁻⁽²ⁿ⁺¹⁾ — each contribution smaller than the last, approaching but never reaching certainty.

### What constitutes "independent convergence"

This is the critical constraint. Adding more Dogs to the same evaluation is NOT independent convergence — it's the same event, same data, Level 0.

| Level | What it requires | Example |
|-------|-----------------|---------|
| 0 → 1 | Same conclusion reached across **different evaluation events** over time | "Sicilian Defense scored HOWL on March 3, March 10, March 17 — always by different Dog configurations" |
| 1 → 2 | Same pattern holds across **different content** in the same domain | "All sound chess openings score HOWL, all traps score BARK — the pattern generalizes" |
| 2 → 3 | Same principle holds across **different domains** | "The axiom scoring is consistent whether applied to chess, code review, or geopolitics" |

Each level requires a qualitatively different kind of evidence. This is why convergence to 1.0 is asymptotic — truly independent evidence becomes harder to obtain at each level.

---

## Score vs Confidence: The Separation

Today CYNIC conflates two concepts:

- **Score**: how good is this content? (Q-Score, 0 to φ⁻¹)
- **Confidence**: how certain are we about this score? (implicitly also bounded at φ⁻¹)

These are independent axes:

```
                        Confidence
                    low ──────────── high
           high  │  "promising     │  "established
    Score        │   but unproven" │   truth"
           low   │  "noise"        │  "established
                 │                 │   weakness"
```

A crystal can be WAG-level quality (score 0.42) with 85.4% confidence (Level 1 convergence). That's different from a single verdict of WAG with 61.8% max confidence.

---

## Verdict Thresholds: The HOWL Fix

### The problem

The current HOWL threshold uses `φ⁻¹ × 0.82 = 0.50679`. The number 0.82 is **not φ-derived**:

```
Closest φ expressions to 0.82:
  1 - φ⁻⁴  = 0.8541  (diff: 0.034)
  φ⁻¹ + φ⁻³ = 0.8541  (diff: 0.034)
  1 - φ⁻³  = 0.7639  (diff: 0.056)

0.82 matches nothing. It is arbitrary.
```

Meanwhile WAG and GROWL are mathematically pure:

```
WAG   > φ⁻²           (= 0.381966)       ✓ φ-derived
GROWL > φ⁻² × φ⁻¹ = φ⁻³ (= 0.236068)    ✓ φ-derived
WAG/GROWL ratio = φ                        ✓ self-similar

HOWL  > φ⁻¹ × 0.82    (= 0.506788)       ✗ NOT φ-derived
HOWL/WAG ratio = 1.327                     ✗ NOT φ
```

### The fix: golden subdivision

Each verdict band is the golden cut of the remaining range:

```
Full range: [0, φ⁻¹]

[0, φ⁻¹] cut at φ⁻¹ of segment → φ⁻²               = 0.382  → WAG    ✓
[0, φ⁻²] cut at φ⁻¹ of segment → φ⁻³               = 0.236  → GROWL  ✓
[φ⁻², φ⁻¹] cut at φ⁻¹ of segment → φ⁻² + φ⁻⁴      = 0.528  → HOWL   ✓ (NEW)
```

The formula: **HOWL > φ⁻² + φ⁻⁴**

Properties:
- `φ⁻² + φ⁻⁴ = 0.527864` (vs old 0.506788)
- As % of max: `(φ⁻² + φ⁻⁴) / φ⁻¹ = 1 + φ⁻² = 85.4%` (vs old 82%)
- Each band boundary ratio to the previous: not φ, but `1 + φ⁻²` — still φ-derived
- The threshold connects to convergence: 85.4% = C(1) — "HOWL-worthy content has crystal-level conviction"

### All thresholds (canonical)

**On the phi-bounded scale (0 to φ⁻¹) — what the code produces:**

```
HOWL  > φ⁻² + φ⁻⁴  = 0.527864
WAG   > φ⁻²        = 0.381966
GROWL > φ⁻³        = 0.236068
BARK  ≤ φ⁻³
```

**As percentage of max confidence (0 to 100%) — what skills use:**

```
HOWL  > 85.4%   (= (1 + φ⁻²) × 100 / φ... no, = (φ⁻² + φ⁻⁴)/φ⁻¹ × 100)
WAG   > 61.8%   (= φ⁻¹ × 100)
GROWL > 38.2%   (= φ⁻² × 100)
BARK  ≤ 38.2%
```

**On the raw 0-100 scale (pre-phi-bounding) — what CRYSTALLIZED-TRUTH.md uses:**

```
HOWL  > 85.4
WAG   > 61.8
GROWL > 38.2
BARK  ≤ 38.2
```

---

## Mapping to CYNIC Architecture

### Consciousness Levels ↔ Convergence Levels

| Consciousness | Convergence | Max Confidence | What |
|---------------|-------------|---------------|------|
| REFLEX (L0) | — | — | Pattern match, no evaluation |
| MICRO (L1) | n=0 | φ⁻¹ = 61.8% | Single Dog verdict |
| MACRO (L2) | n=0 | φ⁻¹ = 61.8% | Multi-Dog consensus (same event) |
| META (L3) | n=1 | 1-φ⁻⁴ = 85.4% | Temporal convergence → Crystal |

### CCM ↔ Convergence

| Crystal State | Observations | Convergence | Confidence Bound |
|---------------|-------------|-------------|-----------------|
| Forming | < F(8)=21 | n=0 | φ⁻¹ = 0.618 |
| Crystallized | ≥ F(8)=21 | n=1 | 1-φ⁻⁴ = 0.854 |
| Canonical | ≥ F(13)=233 | n=2 | 1-φ⁻⁶ = 0.944 |

The confidence bound is the MAXIMUM — the actual confidence is still the running mean of Q-Scores, but the SYSTEM can now express "I've seen this enough times to be more than 61.8% sure."

### Anomaly Detection ↔ φ⁻²

Dog disagreement > φ⁻² (38.2%) on any axiom = anomaly signal. This threshold is already correct — it's the same φ⁻² that bounds the WAG threshold. Anomalies live in the GROWL/BARK zone of agreement.

---

## What This Does NOT Change

- **Q-Score formula**: geometric mean of 6 phi-bounded axioms → phi-bounded result. Unchanged.
- **Score floor**: 0.05. Unchanged.
- **Individual Dog scores**: each Dog returns raw scores, kernel phi-bounds them. Unchanged.
- **6 Axioms**: FIDELITY, PHI, VERIFY, CULTURE, BURN, SOVEREIGNTY. Unchanged.
- **SOVEREIGNTY**: geometric mean like all axioms. No special min() semantics.

---

## Debt Inventory: Where Thresholds Must Be Updated

When implementation happens, these locations all need alignment:

### Code
- `cynic-kernel/src/domain/dog.rs:154` — verdict_kind() HOWL threshold
- `cynic-kernel/src/domain/dog.rs:91` — VerdictKind enum comments
- `cynic-kernel/src/domain/ccm.rs` — Crystal struct + classify()

### Docs (in this repo)
- `docs/CYNIC-CRYSTALLIZED-TRUTH.md:296-302` — ✅ DONE: HOWL_MIN updated to 85.4, axiom count to 6
- `CLAUDE.md:90` — HOWL threshold
- `README.md:53` — HOWL threshold
- `FRONTEND.md:198` — HOWL threshold
- `replit.md:55` — HOWL threshold

### Skills
- `.claude/commands/cynic-kernel.md` — PHI CONSTANTS section
- `.claude/commands/cynic-judge.md` — verdict thresholds
- `.claude/commands/frontend-dev.md` — verdict thresholds
- `~/.claude/commands/cynic-skills/cynic-judge/SKILL.md` — verdict thresholds
- `~/.claude/commands/cynic-skills/cynic-judge/references/dimensions.md` — verdict table
- `~/.claude/commands/cynic-skills/cynic-kernel/SKILL.md` — PHI CONSTANTS section

### External
- `.agents/skills/cynic-judge/SKILL.md` — Gemini agent copy
- `.agents/skills/cynic-judge/references/dimensions.md` — Gemini agent copy

---

## Falsifiability

This crystallization can be falsified if:
1. The HOWL threshold at 0.528 makes the benchmark chess test fail in a way that reveals the old threshold was better calibrated empirically (test with `/test-chess`)
2. The score/confidence separation adds complexity without observable benefit in real evaluations
3. Crystal convergence levels don't produce meaningfully different behaviors than the current flat φ⁻¹ bound

Test before adopting. Math is necessary but not sufficient — empirical validation required.
