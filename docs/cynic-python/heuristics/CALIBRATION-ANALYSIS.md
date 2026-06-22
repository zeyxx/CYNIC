# Token Calibration Analysis (2026-04-29)

## Executive Summary

Synthetic baseline: 77.8% accuracy on 9 tokens (3 BARK, 3 HOWL, 3 GROWL).

**Token domain:** 100% accurate (BARK and HOWL perfectly separated).  
**Twitter/Wallet domains:** Miscalibrate GROWL (ambiguous young tokens) as WAG (marginal tokens).

## The Problem

GROWL tokens (PUMP_LEGIT, NEW_PROTO) score 0.39-0.40 when they should stay ≤0.382 GROWL range.

**Root cause:** Twitter and wallet scorers reward characteristics that are GOOD for young legitimate tokens but also HIGH in young rugs. Specifically:

- **Engagement rate 0.045-0.05**: Excellent for a 5-day-old project. But scorers treat this as a HOWL signal (mature community activity), not a GROWL signal (promising but unproven).
- **Positive sentiment 58-60%**: Real retail believes in the project. But scorers have no GROWL calibration — they only know BARK (rug allegations dominate) vs HOWL (overwhelmingly positive).
- **Retail holding 72-75%**: Community owns the token, not bots. Scorers see this as anti-whale (HOWL) but have no GROWL baseline for "real but concentrated" young tokens.

## The Design Flaw

Twitter and wallet scorers are **domain-agnostic**. They don't know:
- A 5% engagement rate means something different for a token 5 days old vs 500 days old
- A young token with NO rug allegations is NOT equivalent to a mature token with no rug allegations
- "Real retail" on a pump.fun token is different from "real retail" on a listed token

Current fusion:
```
fused = 0.6 * token_score + 0.2 * twitter_score + 0.2 * wallet_score
```

Token domain = GROWL (correct).  
Twitter/Wallet domains = HOWL-ish (miscalibrated).  
Result: fused = 0.401 (WAG, wrong).

## What Real Calibration Will Fix

**Hypothesis:** Real CultScreener data will provide ground truth verdicts for:

1. **Age-stratified GROWL tokens** (verified legitimate by CultScreener but young)
   - Expected signal: How old is "old enough"? Does engagement rate scale with age?
   - Measurement: Correlation between token age (hours) and twitter engagement rate
   - Goal: Identify the age threshold where twitter/wallet signals become independent predictors

2. **Twitter/wallet independence test** (Archimedes principle)
   - Expected signal: Do all three domains agree on real data?
   - Current state: Token domain ✓, Twitter/Wallet ✗
   - Fix: Separate domain-specific heuristics for young tokens (age < 336h = 2 weeks)

3. **Confused signals** (high engagement but no exchange listing)
   - PUMP_LEGIT, NEW_PROTO: Real community but pump.fun origin means high discoverability risk
   - Real data will show: Are pump.fun tokens with real engagement more likely to become established or to die?

## Next Actions

### Phase 1: Real Data Ingestion
```bash
cd cynic-python/heuristics

# When CultScreener is available:
python token_dataset_ingester.py \
  --risk-levels high medium low \
  --count-per-level 20

# Produces:
#   ~/.cynic/datasets/tokens/ground_truth.json (60 tokens, full signals)
#   ~/.cynic/datasets/tokens/ground_truth.csv (summary for manual review)
```

### Phase 2: Measurement + Analysis
```bash
# Run measurement pipeline
python measure_against_ground_truth.py

# Expected output:
#   - Confusion matrix showing which categories fail
#   - Accuracy by ground truth verdict
#   - Domain breakdown (token-only vs fused)
```

### Phase 3: Targeted Tuning (if needed)
If GROWL accuracy < 70%:
1. Identify the failing tokens (which GROWL tokens score as WAG/HOWL?)
2. Extract their age_hours, engagement_rate, positive_pct
3. Plot: Does age correlate with the miscalibration?
4. Hypothesize: "Tokens < N hours old should discount twitter engagement by X%"
5. Implement the age-based adjustment
6. Re-measure

### Phase 4: Deploy to Kernel
Once fused accuracy ≥ 75% on real data:
1. Translate twitter_heuristics.py + wallet_heuristics.py thresholds to Rust
2. Ship in deterministic-dog (kernel-resident, zero external deps)
3. Measure agreement with Dogs on production judgment requests

## Critical Data Hygiene

**DO NOT:**
- Adjust thresholds to fit the mock data (it's synthetic)
- Use GROWL miscalibration as a reason to remove twitter/wallet domains (they're under-calibrated, not broken)
- Skip the age-stratified analysis (age is likely the hidden variable)

**DO:**
- Measure before/after for any threshold change (real data only)
- Keep measurement script and ground-truth dataset in sync (both get versioned)
- Document why each threshold is set (link to ground truth token that drives it)

## Timeline

| Task | Duration | Blocker |
|------|----------|---------|
| Real data ingestion (60 tokens) | 2-3 min | CultScreener API live |
| Measurement (60 tokens) | <1 min | Ingestion done |
| Analysis + hypothesis (if needed) | 10 min | Measurement done |
| Tuning (if accuracy < 75%) | 5-30 min per threshold | Analysis done |
| Kernel translation | 15 min | Thresholds final |
| Deploy + measure agreement | 10 min | Kernel ready |

**Total time to production:** <1 hour (once APIs available).

## References

- QUICKSTART.md — 3-command workflow
- DATAFLOW.md — Full architecture
- token_heuristics.py — Token domain (production-ready baseline)
- twitter_heuristics.py — Twitter domain (needs age-stratified calibration)
- wallet_heuristics.py — Wallet domain (needs age-stratified calibration)
- create_mock_dataset.py — This run's synthetic data (for regression testing)
