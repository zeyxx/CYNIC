# Token Stimulus Protocol — Roadmap

> From grounding → calibrated token judgment.
> Based on: 43-dimension mapping, 9-case calibration corpus, baseline measurement, literature review.
> Date: 2026-04-15

## What Exists Now

| Component | Status | Location |
|-----------|--------|----------|
| Token stimulus builder | DONE | `cynic-kernel/src/domain/stimulus.rs` (build_token_stimulus) |
| TokenData struct | DONE | `cynic-kernel/src/domain/stimulus.rs` (15 fields) |
| Domain prompt (token-analysis) | DONE (not deployed) | `cynic-kernel/domains/token-analysis.md` |
| 43-dimension reference | DONE | `docs/reference/token-43-dimensions.md` |
| Calibration corpus (9 cases) | DONE | `cynic-kernel/tests/calibration_token.rs` |
| Baseline measurement | DONE | `docs/reference/token-calibration-baseline.md` |
| Stimulus module wired | DONE | `domain/mod.rs` includes `stimulus` |

## What Must Happen Next (5 phases)

### Phase 1: Deploy Domain Prompt (1 session, mechanical)
**Hypothesis:** Loading `domains/token-analysis.md` improves JUP from WAG→HOWL and reduces max_disagreement.

Steps:
1. Rebuild kernel with new `stimulus` module and `token-analysis.md`
2. Deploy to production
3. Re-run 3-case calibration (same stimuli)
4. Measure: Q-score delta, max_disagreement delta, per-Dog score changes

**Falsification:** If JUP stays WAG and max_disagreement > 0.40, domain prompt alone is insufficient → proceed to Phase 2.

### Phase 2: Extend TokenData with Research-Grounded Signals (1-2 sessions)
**Hypothesis:** Adding bundle detection, LP flow direction, and holder growth trajectory improves Dog discrimination on ambiguous cases.

New fields for TokenData:
```
bundle_detected: bool            // Jito bundle at launch (earliest signal, arXiv 2602.13480)
day1_tx_concentration: Option<f64>  // % of lifetime txs on day 1 (arXiv 2603.24625)
lp_add_remove_ratio: Option<f64>   // LP adds / LP removes (arXiv 2504.07132, 97.6% accuracy)
holder_growth_7d: Option<f64>      // 7-day holder count change %
creator_wallet_age_hours: Option<u64> // Creator wallet age (not token age)
creator_token_count: Option<u32>   // How many tokens this creator deployed (serial rugger detection)
gini_coefficient: Option<f64>      // Gini over HHI (better for long tails, SoK arXiv 2501.18279)
```

Steps:
1. Extend TokenData struct
2. Update build_token_stimulus to include new fields in METRICS section
3. Update BASELINES with thresholds from literature (τ_down=0.73, day1_concentration>95% = rug)
4. Update calibration corpus with new fields
5. Re-measure baseline

**Falsification:** If new fields don't improve ASDF discrimination (still BARK), the problem is Dog interpretation, not data availability.

### Phase 3: Token-Specific Deterministic Dog (2-3 sessions)
**Hypothesis:** A deterministic scorer for token metrics eliminates dependency on gemini-cli as sole discriminator.

Current deterministic-dog scores FORM (word count, vocab). It provides Δ=0.019 on token domain — essentially noise. A token-specific scorer would:
- Score authority status (binary: revoked=high, active=low)
- Score concentration (HHI/Gini thresholds from literature)
- Score LP status (burned>locked>unsecured)
- Score token age (logarithmic decay of skepticism)
- Score bundle detection (binary)
- Score creator history (serial deployer penalty)

This could be:
a. Extension of deterministic-dog (new domain branch)
b. Separate Dog (token-deterministic-dog)
c. Pre-scoring layer in stimulus builder (enriched baselines)

Option (c) is cheapest — embed deterministic assessments in the stimulus itself so LLM Dogs see "Authority: REVOKED ✓ (safe)" instead of just "REVOKED (supply is fixed)." The LLM then inherits the deterministic judgment.

**Falsification:** If option (c) doesn't improve scores, the LLM Dogs ignore embedded assessments → need option (a) or (b).

### Phase 4: CultScreener Pipeline Integration (1-2 sessions)
**Hypothesis:** Connecting CultScreener (the screener that fetches real Helius data) to the kernel's /judge endpoint creates the E2E flow: on-chain data → stimulus → Dogs → verdict → crystal.

The CultScreener/ directory exists in the repo. This phase wires:
- CultScreener fetches TokenData from Helius (getAsset, getTokenHolders, getAccountInfo)
- Builds stimulus via build_token_stimulus
- Posts to /judge with domain="token-analysis"
- Receives verdict
- Acts on verdict (K15: consumer must ACT — display, gate, alert)

**Falsification:** If round-trip latency >60s, need async pipeline (judge returns verdict_id, screener polls).

### Phase 5: Calibration at Scale (ongoing)
**Hypothesis:** With 100+ token verdicts, per-Dog weighting becomes statistically viable.

Steps:
1. Run CultScreener on known token datasets (SolRugDetector's 100K tokens has ground truth labels)
2. Accumulate per-Dog accuracy on known outcomes
3. Compute per-Dog weights (Dogs that discriminate get higher weight)
4. Crystal formation begins (token-analysis domain crystals)
5. Crystal injection improves future verdicts (the flywheel)

**Falsification:** If per-Dog weighting doesn't improve over equal weighting, Dogs are interchangeable → replace weakest with stronger model.

---

## Critical Path

```
Phase 1 (deploy prompt) ──→ Phase 2 (enrich data) ──→ Phase 4 (E2E pipeline)
                              │
                              └──→ Phase 3 (deterministic scorer) ──→ Phase 5 (scale)
```

Phase 1 is the next session's work. Phase 2 and 3 can run in parallel.
Phase 4 requires Phase 1+2. Phase 5 requires Phase 4.

## Truths Anchoring This Roadmap

| T# | Truth | Confidence | Phase |
|----|-------|-----------|-------|
| T1 | CYNIC is better at BARKing than HOWLing on tokens | 58% | All |
| T2 | Stimulus quality bounds judgment quality | 60% | 1, 2 |
| T3 | 43 dimensions are for cortex, Dogs score 6 axioms | 61% | 1 |
| T4 | Deterministic-dog should lead token scoring | 48% | 3 |
| T5 | Bayesian priors in stimulus are the best calibration tool | 55% | 1, 2 |
| T6 | Helius getTokenHolders is incomplete | 58% | 2, 4 |
| T7 | Ground truth exists from academic datasets | 55% | 5 |
| T8 | Bundle detection is earliest discriminating signal | 52% | 2 |
| T9 | LP add-to-remove ratio = 97.6% accuracy | 50% | 2 |
| T10 | No existing multi-axiom token scoring system | 55% | Differentiation |
