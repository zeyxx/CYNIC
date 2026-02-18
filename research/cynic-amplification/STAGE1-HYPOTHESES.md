# CYNIC Amplification — Stage 1: Hypothesis Formulation

**From**: Stage 0 (Literature Review complete)
**Date**: 2026-02-18

---

## PRIMARY HYPOTHESIS (H1)

**"CYNIC-Amplified small LLM hypothesis"**

> *A small local LLM (qwen2.5-coder:7b, 7B params) augmented with CYNIC's framework
> (persistent memory + 10-agent judgment + Q-Learning RL) will achieve ≥80% of
> Claude Sonnet 4.6's performance on software engineering judgment tasks
> (code quality scoring, bug detection, refactoring suggestions) while operating
> at ≤5% of Claude API cost per task.*

### Formalization

```
H1: Quality(CYNIC[qwen2.5-coder:7b]) ≥ 0.80 × Quality(Claude-Sonnet-4.6-solo)
    AND
    Cost(CYNIC[qwen2.5-coder:7b]) ≤ 0.05 × Cost(Claude-Sonnet-4.6-solo)
```

Where `Quality` is measured by:
- Correlation with human expert judgment (0-100 scale, Kendall's τ)
- Agreement rate on BARK/GROWL/WAG/HOWL verdict categories (Cohen's κ)
- Accuracy on SWE-bench-lite subset (% issues correctly identified)

**Success criterion**: τ ≥ 0.65, κ ≥ 0.60, cost_ratio ≤ 0.05
**Failure criterion**: τ < 0.50 OR κ < 0.45 OR cost_ratio > 0.10

---

## SECONDARY HYPOTHESES

### H2: Memory Dominance
> *Persistent memory (Q-Table warm-up + ScholarDog) contributes >50% of CYNIC's
> total amplification gain vs. base model alone.*

```
H2: Δ_memory / Δ_total ≥ 0.50
    where Δ = Quality(augmented) - Quality(base)
```

**Testable via**: Ablation — CYNIC with memory disabled vs. full CYNIC
**Significance**: If true, ScholarDog persistence (currently non-persistent across sessions)
is the highest-priority fix.

### H3: RL Warm-Up Threshold
> *Q-Learning on verdict sequences shows measurable improvement after ≥50 judgments
> on the same state-key distribution (diminishing returns after 200 judgments).*

```
H3: Quality(CYNIC[n=50 episodes]) > Quality(CYNIC[n=0]) by ≥10%
    AND
    Quality(CYNIC[n=500]) ≤ Quality(CYNIC[n=200]) × 1.05 (diminishing returns)
```

**Significance**: Determines whether CYNIC needs "pre-warming" before deployment,
or if it reaches useful performance immediately.

### H4: Specialization beats Self-MoA
> *CYNIC's 10 specialized Dogs (GUARDIAN, ORACLE, SAGE, SCHOLAR, etc.) outperform
> running a single generic LLM 10 times (Self-MoA baseline) on complex
> multi-dimensional code evaluation tasks.*

```
H4: Quality(CYNIC[10 specialized dogs]) > Quality(Self-MoA[single LLM × 10])
    on tasks requiring simultaneous risk assessment + prediction + temporal scoring
```

**Literature basis**: Self-MoA beats heterogeneous MoA by 6.6% on AlpacaEval,
but AlpacaEval favors general quality. Domain-specific tasks may favor specialization.
**If H4 is FALSE**: Consider collapsing some dogs to reduce complexity (BURN axiom).

### H5: φ-Bounding prevents reward hacking
> *Q-Learning with φ-bounded confidence (≤61.8% max) avoids over-optimization
> and produces more stable long-term learning vs. unconstrained Q-Learning.*

```
H5: Variance(Q-values[φ-bounded]) < Variance(Q-values[unconstrained]) after N episodes
    AND
    Long-term trend of Q[φ-bounded] is monotonically non-decreasing
    while Q[unconstrained] may exhibit reward-hacking spikes
```

**Significance**: Novel theoretical contribution — φ as a regularizer for RL.

---

## BOOTSTRAP LOOP HYPOTHESIS (Meta-level)

Beyond the experimental hypotheses, there is an architectural meta-hypothesis:

**H0 (Meta)**: *The CYNIC Python kernel (current state) serves as sufficient foundation
to bootstrap the full autonomous CYNIC system through 3 phases:*

```
Phase 1 (current):  Bootstrap kernel → proves mechanisms work
Phase 2 (plugin):   Claude Code plugin → CYNIC assists building CYNIC (self-referential)
Phase 3 (target):   Autonomous CYNIC → Ollama-powered, omniscient, omnipotent
```

This is NOT a falsifiable hypothesis in the classical sense — it's an architectural roadmap.
But it has OBSERVABLE MILESTONES:
- M1: First judgment of external code via Claude Code plugin hook → ✓ measured
- M2: First Q-Table state accumulated from real (not test) feedback → unmeasured
- M3: First autonomous ACT executed without human prompt → unmeasured
- M4: Q-Table shows measurable improvement after 1 week of real use → unmeasured

---

## FALSIFIABILITY MATRIX

| Hypothesis | What would CONFIRM | What would REFUTE |
|------------|-------------------|-------------------|
| H1 | τ ≥ 0.65, κ ≥ 0.60 | τ < 0.50 after full augmentation |
| H2 | Memory ablation drops >50% | Memory ablation drops <20% |
| H3 | +10% at 50 episodes | No improvement at 100 episodes |
| H4 | 10 Dogs > Self-MoA on multi-dim | Self-MoA ≥ 10 Dogs on all tasks |
| H5 | φ-bound variance < unbound | No significant variance difference |

---

## RESEARCH DESIGN PREVIEW (Stage 2)

**Primary evaluation dataset**: 100 code snippets sampled from:
- 50 real GitHub Python files (varied complexity)
- 25 synthetic code with known issues (bugs, security flaws, dead code)
- 25 high-quality reference implementations (expected HOWL verdicts)

**Ground truth**: Expert human judgment (you + 1-2 peers) on BARK/GROWL/WAG/HOWL scale
**Primary metric**: Kendall's τ between CYNIC verdict distribution and human judgment
**Cost metric**: Total tokens consumed × cost per token (API) or inference time × GPU cost (local)

**Experimental conditions**:
1. Base: gemma2:2b solo (no CYNIC)
2. Base: qwen2.5-coder:7b solo (no CYNIC)
3. CYNIC + gemma2:2b (memory only)
4. CYNIC + gemma2:2b (memory + 3 dogs)
5. CYNIC + gemma2:2b (full 10 dogs)
6. CYNIC + qwen2.5-coder:7b (full 10 dogs) ← primary
7. CYNIC + qwen2.5-coder:7b (full + RL warm-up 100 episodes)
8. Claude Sonnet 4.6 (ceiling reference)
9. Self-MoA gemma2:2b × 10 (competing baseline)

→ **Checkpoint**: Does this experimental design seem right before proceeding to Stage 2?

---

*Stage 1 complete — awaiting Stage 2 approval.*
