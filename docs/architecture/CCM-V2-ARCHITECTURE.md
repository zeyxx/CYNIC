# CCM v2 — Architecture Decision Record

> Crystal Coherence Machine redesign based on empirical audit + external research.
> Date: 2026-05-15. Status: PROPOSAL (not implemented).

---

## Problem Statement

The CCM v1 conflates three distinct concerns under one name:

| Concern | What v1 does | What the data shows |
|---------|-------------|---------------------|
| **Memory** | Crystal DB + RAG injection into Dog prompts | 70% noise (commits, session logs). Effect unmeasured. |
| **Learning** | Nothing (LoRA planned, never built) | Zero training pipeline exists |
| **Consensus** | Geometric mean + phi-bound across Dogs | 0.985 cosine between Dogs. 1 effective dimension. |

Audit data (2026-05-12, 120K observations, 13K verdicts):
- 73% BARK, 0% HOWL outside chess — positive signal truncated
- Dogs quasi-identical (cosine 0.985-0.997)
- 6 axioms collapsed to 1 effective dimension (PC1=80%)
- Crystal injection effect: unmeasured

The compound loop is a **positive feedback loop toward suspicion** (model collapse via recursive self-consumption, Nature 2024).

---

## Meta-Principle (D11)

**Calibrated feedback at every scale.**

Every loop in the organism must have a measurement that distinguishes improvement from collapse. A loop without measurement is an echo chamber by default.

Applied fractally:
- Per-crystal: confidence bounded at phi-inv
- Per-Dog: role separation prevents convergence
- Per-loop: anchor corpus prevents tail truncation
- Per-system: A/B test measures actual improvement

This generates most other adopted patterns: circuit breakers (calibrated failure threshold), source gate (calibrated content filter), phi-bound (calibrated confidence ceiling), crystal challenge (calibrated quality gate).

---

## The Three Systems

CCM v2 separates the three concerns into distinct subsystems with clear interfaces.

### System 1: JURY (Consensus)

**Problem it solves:** Multiple independent judges must produce a single calibrated score.

**What v1 gets wrong:** All Dogs score all 6 axioms with the same prompt. Result: cosine 0.985+ between Dogs. Adding Dogs adds volume, not perspective.

**Research basis:**
- AIME (2025): exclusive criterion per judge outperforms shared scoring
- Negative Correlation Learning (ICLR 2025): SFT actively destroys diversity
- Locally weighted aggregation (IJCAI 2025): down-weight correlated judges
- Rogan-Gladen estimator: correct for judge bias with small gold set

**v2 design:**

```
Dog A (FIDELITY + VERIFY)     ──┐
Dog B (PHI + BURN)            ──┼──► Weighted aggregation ──► Verdict
Dog C (CULTURE + SOVEREIGNTY) ──┘         │
Deterministic (FORM metrics)  ──────────┘
                                    │
                              Correlation monitor
                              (alert if cosine > 0.9)
```

Each inference Dog owns 2 axioms exclusively. Deterministic-dog remains cross-cutting (form metrics). Aggregation down-weights Dogs that correlate above threshold.

**Falsify:** If role-separated Dogs produce LOWER inter-rater agreement than shared-scoring Dogs on a held-out test set, role separation hurts more than it helps.

**Interface:**
```
Jury::evaluate(stimulus) -> Verdict {
    q_score: AxiomScores,      // per-axiom, not just total
    agreement: f64,             // inter-Dog agreement
    voter_count: usize,
    diversity_score: f64,       // NEW: how independent were the judges
}
```

### System 2: MEMORY (Crystal Store + Retrieval)

**Problem it solves:** The organism must remember verified knowledge across sessions without poisoning itself.

**What v1 gets wrong:** No source filter (fixed today), no anchor corpus, no measurement of injection effect, no structured forgetting.

**Research basis:**
- Model collapse (Nature 2024): never train exclusively on own outputs
- Zep/Graphiti: temporal knowledge graph with valid_at/invalid_at
- AMA (2026): judge-gated writes
- D-MEM (2026): dopamine-gated (surprise + utility)
- Agent drift (arXiv 2601): critical threshold at ~300 interactions

**v2 design:**

```
                    ┌─────────────────────────────────┐
                    │         ANCHOR CORPUS            │
                    │  (external ground truth,         │
                    │   human-curated, immutable)       │
                    └──────────┬──────────────────────┘
                               │
Verdict ──► Source gate ──► Write gate ──► Crystal DB
            (domain          (consensus       │
             whitelist)       > phi-inv)       │
                                              │
                              ┌───────────────┤
                              │               │
                         KNN search      Anchor mix
                              │               │
                              └───────┬───────┘
                                      │
                              Crystal context
                              (injected into
                               Dog prompts)
                                      │
                              ┌───────┴───────┐
                              │  A/B MEASURE   │
                              │  (with vs      │
                              │   without)     │
                              └───────────────┘
```

**Anti-collapse mechanisms (3 layers):**

1. **Source gate** (implemented today): only external-content domains crystallize
2. **Anchor corpus**: fixed set of known-good verdicts mixed into retrieval (prevents tail truncation). Ratio: at least 20% anchor, 80% organic crystals.
3. **A/B measurement**: every Nth request runs without crystal injection. Track Q-score delta. If delta <= 0, crystals are not helping — STOP injection until diagnosed.

**Structured forgetting:**
- Crystals carry `valid_at` / `challenged_at` timestamps
- TTL based on observation recency, not count
- crystal_challenge runs on crystallized/canonical, dissolves on Q-delta > phi-inv-2
- Forming crystals that don't reach crystallized within 30 days: auto-dissolved

**Interface:**
```
Memory::retrieve(stimulus_embedding, domain) -> CrystalContext {
    organic_crystals: Vec<Crystal>,    // from DB, KNN matched
    anchor_crystals: Vec<Crystal>,     // from corpus, always present
    injection_enabled: bool,           // A/B flag
}

Memory::observe(verdict, domain) -> Result<(), GateRejection> {
    // Source gate + consensus gate + diversity check
}
```

### System 3: DISTILLATION (Learning Pipeline)

**Problem it solves:** The organism must get genuinely better over time, not just accumulate data.

**What v1 has:** Nothing. LoRA planned since March 2026, never built.

**Research basis:**
- Constitutional AI (Anthropic): axioms as teacher signal, RLAIF two-phase
- RM-Distiller (2025): continuous scores from ensemble → student training
- Pessimistic distillation: student underestimates on uncertain examples
- Time-Varying LoRA (NeurIPS 2024): continuous parameter manifold
- DCLM: curation quality dominates over data volume
- unsloth (starred): practical LoRA path for Qwen3/Gemma4

**v2 design:**

```
Crystal DB (verified, crystallized+canonical only)
    │
    ▼
Training data curator
    │
    ├── Consensus filter: agreement > phi-inv
    ├── Distribution balance: not >60% any single verdict kind
    ├── Domain diversity: min 3 domains represented
    │
    ▼
Instruction pairs: (stimulus, axiom_scores, verdict_kind)
    │
    ▼
LoRA fine-tune (batch, weekly or on N new crystals)
    │
    ├── Base model: Qwen 2.5 7B (sovereign, on cynic-core)
    ├── Tool: unsloth
    ├── Validation: held-out crystal set (20% split)
    │
    ▼
Candidate Dog
    │
    ├── Shadow mode: runs alongside existing Dogs, not counted in verdict
    ├── Agreement tracking: does it agree with consensus?
    ├── After M verdicts: promote to active Dog or reject
    │
    ▼
Production Dog (replaces or supplements existing)
```

**Guard rails:**
- Never train on own outputs exclusively (Nature 2024 collapse). Always mix anchor corpus.
- Batch retrain (weekly/N crystals), not online — prevents catastrophic forgetting.
- Shadow mode before promotion — candidate Dog must demonstrate calibrated agreement.
- Pessimistic: train student to underestimate on low-consensus verdicts.

**Interface:**
```
Distillation::curate() -> TrainingSet {
    // Reads crystallized+canonical, applies quality gates
    // Returns instruction pairs ready for LoRA
}

Distillation::train(training_set) -> CandidateDog {
    // Runs LoRA fine-tune, returns model for shadow evaluation
}

Distillation::evaluate(candidate, held_out) -> PromotionDecision {
    // Shadow-run candidate against held-out set
    // Promote / Reject / Continue shadow
}
```

---

## What Changes in v2

| v1 | v2 | Why |
|----|-----|-----|
| All Dogs score all 6 axioms | Each Dog owns 2 axioms (role separation) | Prevents cosine 0.985 convergence |
| Geometric mean, uniform weights | Locally weighted, correlation-penalized | Down-weights redundant Dogs |
| No source filter on crystals | Source gate (whitelist) | Prevents 70% noise crystallization |
| No anchor corpus | 20% anchor mix in retrieval | Prevents tail truncation / model collapse |
| No A/B measurement | Every Nth request runs without injection | Measures actual crystal effect |
| No training pipeline | Weekly LoRA from curated crystals | Closes the learning loop |
| Crystal challenge only checks Q-delta | Challenge + TTL + structured forgetting | Multiple quality mechanisms |
| "CCM" = one monolith | Jury + Memory + Distillation = 3 systems | Clear interfaces, independent evolution |

---

## Build Sequence

### Phase 0: Measure (before ANY code)

**Observable:** A/B data showing crystal injection effect (positive, neutral, or negative).

1. Wire `inject_crystals` toggle to alternate every Nth request
2. Log both paths with same stimulus
3. After 100+ paired comparisons: compute delta
4. If delta <= 0: STOP crystal injection. Fix Memory before proceeding.
5. If delta > 0: quantify the gain. Proceed to Phase 1.

**Why first:** Every subsequent phase assumes crystals help. If they don't, the entire v2 is solving the wrong problem. Measure before building.

### Phase 1: Anti-Collapse (Memory hardening)

**Observable:** Crystal domain distribution shifts from 73% BARK toward measured baseline.

1. Build anchor corpus (10-20 known-good verdicts per domain, human-curated)
2. Mix anchors into crystal retrieval (20% floor)
3. Add structured forgetting (TTL on forming crystals, 30d auto-dissolve)
4. Source gate already implemented (PR #176)

### Phase 2: Role Separation (Jury redesign)

**Observable:** Inter-Dog cosine drops below 0.9. Effective dimensions increase from 1 to 3+.

1. Assign axiom pairs per Dog in backends.toml
2. Modify prompt builder to include only assigned axioms
3. Implement correlation monitor (alert on cosine > 0.9)
4. Implement locally weighted aggregation

**Dependency:** Requires 3+ active Dogs. Currently 2. Sovereign Dog restart is prerequisite.

### Phase 3: Distillation (Learning pipeline)

**Observable:** A LoRA-tuned Dog passes shadow evaluation and is promoted to production.

1. Build training data curator (crystals → instruction pairs)
2. Integrate unsloth for LoRA fine-tune on cynic-gpu
3. Build shadow evaluation framework
4. First training run on chess domain (most data, clearest signal)
5. If chess-Dog passes: extend to token-analysis domain

**Dependency:** Phase 1 (clean crystals) + Phase 2 (diverse verdicts). Cannot train on collapsed data.

### Phase 4: Compound Loop (close the loop)

**Observable:** Organism judgment quality improves measurably over 30 days without human intervention.

1. Distilled Dog produces verdicts → new crystals accumulate
2. New crystals feed next distillation cycle
3. A/B measurement confirms each cycle improves (not collapses)
4. Anchor corpus updated with human-verified best verdicts from each cycle

**This is the compound loop done right:** real-data anchoring + measurement at every step.

---

## Rejected Alternatives

| Alternative | Why rejected |
|------------|-------------|
| Online LoRA (continuous training) | Catastrophic forgetting risk without replay buffers. Batch is safer. |
| Single-Dog judge (AMA pattern) | Loses the diversity signal. Multi-Dog is the structural advantage. |
| Kill crystals entirely (Loop B only) | Loop B (distill → CLAUDE.md → hooks) works but is human-bottlenecked. Loop A scales. |
| Unbounded confidence | phi-inv ceiling is honest. Removing it would mask calibration problems. |
| Content-based filter (regex/heuristic) | Fragile. Source/domain-based gate is more robust. |

## Deferred Decisions

| Decision | Resolve when |
|---------|-------------|
| Which 2 axioms per Dog | After Phase 2 correlation analysis on 500+ verdicts with current shared scoring |
| Crystal graph (Zep-style temporal) vs flat DB | After Phase 1 TTL shows whether temporal queries are needed |
| Darwinian prompt evolution (from stars) | After Phase 3 proves LoRA works. Mutation before selection = noise. |
| Domain-specific aggregation weights | After Phase 2 produces per-domain calibration data |

---

## Sources

### Academic
- Shumailov et al., "AI models collapse when trained on recursively generated data," Nature 631 (2024)
- "Is Model Collapse Inevitable?" arXiv 2404.01413
- "Degenerate Feedback Loops in Recommender Systems," arXiv 1902.10730
- "Agent Drift: Quantifying Behavioral Degradation in Multi-Agent LLM Systems," arXiv 2601.04170
- "AIME: Exclusive criterion assignment for multi-LLM judges," 2025
- "Negative Correlation Learning preserves diversity," ICLR 2025
- "Wisdom of the Silicon Crowd," Science Advances 2024
- "Locally weighted aggregation for bias mitigation," IJCAI 2025
- "Rogan-Gladen estimator for LLM-as-judge calibration," arXiv 2511.21140
- "RM-Distiller: Reward model distillation from LLM ensemble," arXiv 2601.14032
- "Time-Varying LoRA," NeurIPS 2024
- "Constitutional AI: Harmlessness from AI Feedback," Anthropic 2022
- "DCLM: Data-centric language model pre-training," 2024

### Internal
- CCM-ORGANISM-AUDIT-2026-05-12: 120K observations, 13K verdicts, 1 effective dimension
- CCM-COMPOUND-PROTOCOL: Loop A vs Loop B distinction
- CCM-PRODUCT-CRYSTALLIZATION: original vision (knowledge into weights)
- CYNIC CLAUDE.md: "CCM is the only guaranteed inter-session memory"

### Prior Art (GitHub stars)
- thedotmack/claude-mem: compression + injection analog
- karpathy/autoresearch: self-improving loop on single-GPU
- imbue-ai/darwinian_evolver: evolutionary prompt optimization
- ouroboros (2 forks): self-creating agent loop
- unslothai/unsloth: practical LoRA path

---

*The organism is not complete. It is calibrated at its current scale.*
